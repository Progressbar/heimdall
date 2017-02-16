extern crate pn532;
extern crate mifare;
extern crate heimdall_db;
extern crate i2cdev;
extern crate rusqlite;
extern crate void;
extern crate rand;
extern crate sysfs_gpio;

pub mod error;
pub mod tags;

use pn532::PN532 as GenericPN532;

type PN532 = GenericPN532<pn532::bus::BusyWait<i2cdev::linux::LinuxI2CDevice>>;

trait SliceToFixed {
    fn to_fixed6(&self) -> Option<&[u8; 6]>;
    fn to_fixed16(&self) -> Option<&[u8; 16]>;
}

impl SliceToFixed for [u8] {
    fn to_fixed6(&self) -> Option<&[u8; 6]> {
        unsafe {
            if self.len() >= 6 {
                Some(std::mem::transmute(self.as_ptr()))
            } else {
                None
            }
        }
    }

    fn to_fixed16(&self) -> Option<&[u8; 16]> {
        unsafe {
            if self.len() >= 16 {
                Some(std::mem::transmute(self.as_ptr()))
            } else {
                None
            }
        }
    }
}

fn dump_tags(conn: &mut rusqlite::Connection) -> Result<(), rusqlite::Error> {
    let mut stmt = try!(conn.prepare(
            "SELECT tag_id, uid, auth_method, auth_data FROM tags"
    ));
    println!("Known tags:");
    let rows = try!(stmt.query_map(&[], |row| {
        let tag_id = row.get::<_, Vec<u8>>(0);
        for b in tag_id {
            print!("{:02X}", b);
        }
        print!("|{}|{}|", row.get::<_, i64>(1), row.get::<_, i64>(2));
        let auth_data = row.get::<_, Option<Vec<u8>>>(3);
        match auth_data {
            None => println!("None"),
            Some(auth_data) => {
                for b in auth_data {
                    print!("{:02X}", b);
                }
                println!("");
            }
        }
    }));
    for _ in rows {}
    Ok(())
}

fn add_tag(uid: u32, conn: &mut rusqlite::Connection, device: &mut PN532) -> Result<(), rusqlite::Error> {
    use std::borrow::Cow;
    use pn532::tags::TagBuffer;
    use mifare::{MifareTag, SectorNumber1K, BlockOffset};
    use error::TagError;
    use rand::OsRng;
    use rand::Rng;

    // TODO: as param
    let sector_number = SectorNumber1K::new(1).unwrap();

    println!("Put your NFC token near antenna");

    let mut tag_buf = TagBuffer::new();
    let mut tag = tags::wait_tag(device, &mut tag_buf)
        .map_err(TagError::Comm)
        .and_then(|t| MifareTag::new(t).ok_or(TagError::InvalidTag)).unwrap();

    let mut auth_data: [u8; 23] = [sector_number.into(), 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

    let mut rng = OsRng::new().unwrap();
    rng.fill_bytes(&mut auth_data[1..]);

    let mut tag_id = [0u8; 7];
    tag_id[..tag.tag_id().len()].copy_from_slice(tag.tag_id());
    let tag_id = &tag_id[0..tag.tag_id().len()];
    let mut tag_info = heimdall_db::Tag {
        id: Cow::Borrowed(tag_id),
        uid: uid,
        auth_method: 0,
        auth_data: Cow::Borrowed(&auth_data),
    };

    static EMPTY_KEY: [u8; 6] = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
    // First we try to authenticate
    let mut sector = tag.authenticate_sector(sector_number, mifare::KeyOption::KeyA, &EMPTY_KEY).unwrap();
    // Store the values into database, so if the operation fails, we can recover the tag.
    try!(tag_info.insert(conn));

    // Write secret data        
    sector.write_block(BlockOffset::new(0).unwrap(), auth_data[7..].to_fixed16().unwrap()).unwrap();

    let mut sector_trailer = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x07, 0x80, 0x69, 0xFF, 0xFE, 0xFF, 0xFF, 0xFF, 0xFF];
    sector_trailer[0..6].copy_from_slice(&auth_data[1..7]);
    sector.write_keys(&sector_trailer).unwrap();

    println!("Key overwritten. Backup the database NOW!");

    tag_info.auth_method = 1;
    tag_info.update(conn)
}

fn main() {
    use mifare::{MifareTag, SectorNumber1K, BlockOffset};
    use pn532::tags::TagBuffer;
    use error::{TagError, AuthError, DatabaseError};
    use sysfs_gpio::{Direction, Pin};

    let block_number = BlockOffset::new(0).unwrap();

    let mut device = tags::setup().unwrap();

    let mut sqlconn = rusqlite::Connection::open("users.db").unwrap();

    let mut args = std::env::args();
    args.next().unwrap();
    args.next().map(|o| match &*o {
        "--init" => heimdall_db::create_tables(&mut sqlconn).unwrap(),
        "--dump-tags" => dump_tags(&mut sqlconn).unwrap(),
        "--add-tag" => add_tag(args.next().expect("missing argument").parse().expect("ivalid argument (has to be integer)"), &mut sqlconn, &mut device).unwrap(),
        _ => (),
    });

    let mut reinit = false;

    let mut tag_buf = TagBuffer::new();
    let relay = Pin::new(21);
    relay.with_exported(|| {
        relay.set_direction(Direction::Out).unwrap();
        loop {
            use std::io::Write;

            if reinit {
                std::thread::sleep(::std::time::Duration::from_millis(60_000));
                device = tags::setup().unwrap();
                reinit = false;
            }

            let tag = tags::wait_tag(&mut device, &mut tag_buf)
                .map_err(TagError::Comm)
                .and_then(|t| MifareTag::new(t).ok_or(TagError::InvalidTag));

            let mut tag  = match tag {
                Ok(tag) => tag,
                Err(TagError::Comm(e)) => {
                    let _ = writeln!(std::io::stderr(), "Warning communication failed: {}", e);
                    reinit = true;
                    continue;
                },
                // Silently ignore bad tags
                Err(TagError::InvalidTag) => continue,
            };

            print!("Found tag with id: ");
            for b in tag.tag_id() {
                print!("{:02X} ", b);
            }
            println!("");

            let mut id = [0u8; 32];
            let tid_len = tag.tag_id().len();
            if tid_len > 32 {
                println!("Tag has too long ID ({}B)", tid_len);
                continue;
            }
            id[0..tid_len].copy_from_slice(tag.tag_id());
            let id = &id[..tid_len];

            let user = heimdall_db::identify_user(&mut sqlconn, id, |_, data| {
                if data.len() != 1 + 6 + 16 {
                    return Err(AuthError::Other(DatabaseError::InvalidLength));
                }

                let sector_number = try!(SectorNumber1K::new(data[0]).ok_or(AuthError::Other(DatabaseError::InvalidSector)));

                let mut rbuf = [0u8; 16];
                let mut sector = try!(tag.authenticate_sector(sector_number, mifare::KeyOption::KeyA, data[1..7].to_fixed6().unwrap()));
                try!(sector.read_block(block_number, &mut rbuf));
                if rbuf == data[7..] {
                    Ok(())
                } else {
                    Err(AuthError::InvalidCredentials)
                }
            });

            match user {
                Ok(user) => {
                    println!("Identified user {}", user.uid);
                    if let Err(e) = relay.set_value(1) {
                        println!("Oh shit, I can't open: {}", e);
                    }
                    std::thread::sleep(::std::time::Duration::from_millis(5000));
                    if let Err(e) = relay.set_value(0) {
                        println!("Oh shit, I can't close: {}", e);
                    }
                },
                Err(e) => println!("Error: {}", e),
            }
        }
    }).unwrap();
}
