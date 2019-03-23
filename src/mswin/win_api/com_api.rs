use lazy_static::lazy_static;
use std::sync::{Mutex,Arc};
use std::ptr::null_mut;
use log::{trace,warn};
use failure::{Fail};
use std::io::Error;


use winapi::{
    shared::{
        ntdef::NULL,
        rpcdce::{
            RPC_C_AUTHN_LEVEL_DEFAULT,RPC_C_IMP_LEVEL_IMPERSONATE,
        },
    },
    um::{
        combaseapi::{
            CoInitializeEx, CoInitializeSecurity, CoSetProxyBlanket, CoUninitialize,
        },
        objbase::COINIT_MULTITHREADED,
        objidl::EOAC_NONE,
    },
};

use crate::{MigError, MigErrorKind, MigErrCtx};
// use super::util::{check_hres};

const MODULE: &str = "mswin::win_api::com_api";

pub type HComApi = Arc<Mutex<Option<ComAPI>>>;

pub struct ComAPI {  }

pub fn get_com_api() -> Result<HComApi,MigError> {
    lazy_static! {
        static ref COM_REF: HComApi = Arc::new(Mutex::new(None));             
    }
    
    if let Ok(mut oca) = (*COM_REF).lock() {
        if let None = *oca {
            trace!("{}::get_com_api: initializing com", MODULE);
            if unsafe { CoInitializeEx(null_mut(), COINIT_MULTITHREADED) } < 0 {
                let os_err = Error::last_os_error();
                warn!("{}::get_com_api: CoInitializeEx returned os error: {:?} ", MODULE, os_err);       
                return Err(
                    MigError::from(
                        os_err.context(
                            MigErrCtx::from_remark(MigErrorKind::WinApi, &format!("{}::get_com_api: CoInitializeEx failed",MODULE)))));
            }
            if unsafe { 
                CoInitializeSecurity(
                    NULL,
                    -1, // let C    OM choose.
                    null_mut(),
                    NULL,
                    RPC_C_AUTHN_LEVEL_DEFAULT,
                    RPC_C_IMP_LEVEL_IMPERSONATE,
                    NULL,
                    EOAC_NONE,
                    NULL,) } < 0 {                    
                let os_err = Error::last_os_error();
                unsafe { CoUninitialize() };                                        
                warn!("{}::get_com_api: CoInitializeSecurity returned os error: {:?} ", MODULE, os_err);       
                return Err(
                    MigError::from(
                        os_err.context(
                            MigErrCtx::from_remark(MigErrorKind::WinApi, &format!("{}::get_com_api: CoInitializeSecurity failed",MODULE)))));                    
                }            

            *oca = Some(ComAPI{});
        }

        Ok(COM_REF.clone())
    } else {
        Err(MigError::from_remark(MigErrorKind::MutAccess, &format!("{}::get_com_api: failed to lock mutex",MODULE)))
    }        
}

impl Drop for ComAPI {
    fn drop(&mut self) {
        trace!("{}::drop: deinitializing com", MODULE);
        unsafe { CoUninitialize() };
    }
}


