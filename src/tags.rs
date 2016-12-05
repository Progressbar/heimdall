use ::error::{SetupError, TagError, CommError};
use ::pn532::PN532;
use ::pn532::tags::{ISO14443AListOptions, ISO14443A};
use ::pn532::tags::TagBuffer;
use ::pn532::bus::BusyWait;

pub type Tag<'b, 'p> = ::pn532::tags::Tag<'p, 'b, ISO14443A<'b>, PN532<BusyWait<::i2cdev::linux::LinuxI2CDevice>>>;

pub fn setup() -> Result<PN532<BusyWait<::i2cdev::linux::LinuxI2CDevice>>, SetupError> {
    use ::pn532::bus::i2c;
    use ::pn532::bus::BusyWait;
    use ::pn532::SAMMode;

    let i2c = try!(i2c::open("/dev/i2c-0"));
    let mut device = PN532::new(BusyWait::new(i2c));
    try!(device.sam_configure(SAMMode::Normal(None)));
    Ok(device)
}

pub fn wait_tag<'b, 'p>(reader: &'p mut PN532<BusyWait<::i2cdev::linux::LinuxI2CDevice>>, buffer: &'b mut TagBuffer) -> Result<Tag<'b, 'p>, CommError> {

    let list_opts = ISO14443AListOptions {
        limit: ::pn532::tags::TagNumLimit::One,
        uid: None
    };

    let tags = try!(reader.list_tags(list_opts, buffer));
    Ok(tags.first())
}
