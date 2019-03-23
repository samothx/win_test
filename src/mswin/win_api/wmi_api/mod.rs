use failure::Fail;
use log::{debug, warn};
use std::io::Error;
use std::ptr::{self, null_mut};
use winapi::{
    shared::{
        ntdef::NULL,
        rpcdce::{
            RPC_C_AUTHN_LEVEL_CALL, RPC_C_AUTHN_WINNT, RPC_C_AUTHZ_NONE,
            RPC_C_IMP_LEVEL_IMPERSONATE,
        },
        wtypesbase::CLSCTX_INPROC_SERVER,
    },    
    um::{
        oaidl::SAFEARRAY,
        objidl::EOAC_NONE,
        combaseapi::{
                    CoCreateInstance, 
                    CoSetProxyBlanket
                    },                
        wbemcli::{  IEnumWbemClassObject,
                    IWbemClassObject,
                    CLSID_WbemLocator, 
                    IID_IWbemLocator, 
                    IWbemLocator, 
                    IWbemServices, 
                    WBEM_FLAG_FORWARD_ONLY, 
                    WBEM_FLAG_RETURN_IMMEDIATELY,
                    WBEM_FLAG_ALWAYS, 
                    WBEM_FLAG_NONSYSTEM_ONLY,
                    },
    },
};

use super::com_api::{get_com_api, HComApi};
use super::util::to_wide_cstring;
use crate::{MigErrCtx, MigError, MigErrorKind};

mod iwbem_class_wr;
pub use iwbem_class_wr::IWbemClassWrapper;

type PMIWbemLocator = *mut IWbemLocator;
type PMIWbemServices = *mut IWbemServices;

const MODULE: &str = "mswin::win_api::wmi_api";

pub struct WmiAPI {
    com_api: HComApi,
    p_loc: PMIWbemLocator,
    p_svc: PMIWbemServices,
}

impl<'a> WmiAPI {
    pub fn get_api() -> Result<WmiAPI, MigError> {
        WmiAPI::get_api_from_hcom(get_com_api()?)
    }

    pub fn get_api_from_hcom(h_com_api: HComApi) -> Result<WmiAPI, MigError> {
        debug!("{}::get_api_from_hcom: Calling CoCreateInstance for CLSID_WbemLocator", MODULE);

        let mut p_loc = NULL;

        if unsafe {
            CoCreateInstance(
                &CLSID_WbemLocator,
                null_mut(),
                CLSCTX_INPROC_SERVER,
                &IID_IWbemLocator,
                &mut p_loc,
            )
        } < 0
        {
            let os_err = Error::last_os_error();
            warn!(
                "{}::get_api_from_hcom: CoCreateInstance returned os error: {:?} ",
                MODULE, os_err
            );
            return Err(MigError::from(os_err.context(MigErrCtx::from_remark(
                MigErrorKind::WinApi,
                &format!("{}::get_api_from_hcom: CoCreateInstance failed", MODULE),
            ))));
        }

        debug!("{}::get_api_from_hcom: Got locator {:?}", MODULE, p_loc);

        debug!("{}::get_api_from_hcom: Calling ConnectServer", MODULE);

        let mut p_svc = null_mut::<IWbemServices>();

        if unsafe {
            (*(p_loc as PMIWbemLocator)).ConnectServer(
                to_wide_cstring("ROOT\\CIMV2").as_ptr() as *mut _,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                0,
                ptr::null_mut(),
                ptr::null_mut(),
                &mut p_svc,
            )
        } < 0
        {
            let os_err = Error::last_os_error();
            warn!(
                "{}::get_api_from_hcom: ConnectServer returned os error: {:?} ",
                MODULE, os_err
            );
            return Err(MigError::from(os_err.context(MigErrCtx::from_remark(
                MigErrorKind::WinApi,
                &format!("{}::get_api_from_hcom: ConnectServer failed", MODULE),
            ))));
        }

        debug!("{}::get_api_from_hcom: Got services {:?}",MODULE, p_svc);

        let wmi_api = Self {
            com_api: h_com_api,
            p_loc: p_loc as PMIWbemLocator,
            p_svc: p_svc,
        };

        debug!("{}::get_api_from_hcom: Calling CoSetProxyBlanket", MODULE);

        if unsafe {
            CoSetProxyBlanket(
                wmi_api.p_svc as _,          // Indicates the proxy to set
                RPC_C_AUTHN_WINNT,           // RPC_C_AUTHN_xxx
                RPC_C_AUTHZ_NONE,            // RPC_C_AUTHZ_xxx
                null_mut(),                  // Server principal name
                RPC_C_AUTHN_LEVEL_CALL,      // RPC_C_AUTHN_LEVEL_xxx
                RPC_C_IMP_LEVEL_IMPERSONATE, // RPC_C_IMP_LEVEL_xxx
                NULL,                        // client identity
                EOAC_NONE,                   // proxy capabilities
            )
        } < 0
        {
            let os_err = Error::last_os_error();
            warn!(
                "{}::get_api_from_hcom: CoSetProxyBlanket returned os error: {:?} ",
                MODULE, os_err
            );
            return Err(MigError::from(os_err.context(MigErrCtx::from_remark(
                MigErrorKind::WinApi,
                &format!("{}::get_api_from_hcom: CoSetProxyBlanket failed", MODULE),
            ))));
        }
        debug!("{}::get_api_from_hcom: Done", MODULE);
        Ok(wmi_api)
    }

    pub fn raw_query(&mut self, query: &str) -> Result<(),MigError> {
        debug!("{}::raw_query: entered with {}", MODULE, query);
        let query_language = to_wide_cstring("WQL");
        let query = to_wide_cstring(query);

        let mut p_enumerator = NULL as *mut IEnumWbemClassObject;

        if unsafe {
            (*self.p_svc).ExecQuery(
                query_language.as_ptr() as *mut _,
                query.as_ptr() as *mut _,
                (WBEM_FLAG_FORWARD_ONLY | WBEM_FLAG_RETURN_IMMEDIATELY) as i32,
                ptr::null_mut(),
                &mut p_enumerator,
            ) } < 0 {
            let os_err = Error::last_os_error();
            warn!(
                "{}::raw_query: ExecQuery returned os error: {:?} ",
                MODULE, os_err
            );
            return Err(MigError::from(os_err.context(MigErrCtx::from_remark(
                MigErrorKind::WinApi,
                &format!("{}::raw_query: ExecQuery failed", MODULE),
            ))));
        }

        debug!("{}::raw_query: Got enumerator {:?}", MODULE, p_enumerator);

        //Ok(QueryResultEnumerator::new(self, p_enumerator))

        Err(MigError::from(MigErrorKind::NotImpl))
    }
}