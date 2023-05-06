use anyhow::Result;
use platform_info::{PlatformInfo, Uname};

pub fn get_platform_hash() -> Result<String> {
    let uname = PlatformInfo::new()?;
    let os_info = os_info::get();
    let distro = os_info.os_type();
    let info_array = [distro.to_string(), uname.release().to_string(), uname.machine().to_string(), uname.osname().to_string()];
    let info_str = info_array.iter().fold(uname.sysname().to_string(), |acc, attr| {
        return format!("{acc}||{attr}")
    });
    let hash = md5::compute(info_str);
    return Ok(format!("{:x}", hash));
}
