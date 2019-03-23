use failure::{ResultExt};
use crate::{MigError, MigErrorKind, MigErrCtx};

pub fn to_string(os_str_buf: &[u16]) -> Result<String,MigError> {            
    match os_str_buf.iter().position(|&x| x == 0 ) {        
        Some(i) => Ok(String::from_utf16_lossy(&os_str_buf[0..i])),
        None => return Err(MigError::from(MigErrorKind::InvParam)),
    }
}

pub fn to_string_list(os_str_buf: &[u16]) -> Result<Vec<String>,MigError> {            
    let mut str_list: Vec<String> = Vec::new();
    let mut start: usize = 0;
    for curr in os_str_buf.iter().enumerate() {
        if *curr.1 == 0 {
            if  start < curr.0 {
                let s = to_string(&os_str_buf[start .. curr.0 + 1]).context(MigErrCtx::from(MigErrorKind::InvParam))?;
                str_list.push(s);
                start = curr.0 + 1;
            } else {
                break;
            }            
        }
    }
    Ok(str_list)
}

pub fn clip<'a>(clip_str: &'a str, clip_start: Option<&str>, clip_end: Option<&str>) -> &'a str {            
    let mut work_str = clip_str;

    if let Some(s) = clip_start {
        if !s.is_empty() && work_str.starts_with(s) {        
            work_str = &work_str[s.len()..];
        }
    }

    if let Some(s) = clip_end {
        if !s.is_empty() && work_str.ends_with(s) {
            work_str = &work_str[0..(work_str.len()- s.len())];
        }
    }

    work_str
}
