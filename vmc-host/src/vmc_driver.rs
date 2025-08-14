use postcard_rpc::{
    header::VarSeqKind,
    host_client::{HostClient, HostErr},
    standard_icd::{PingEndpoint, WireError, ERROR_PATH},
};
use std::convert::Infallible;

use vmc_icd::*;

#[derive(Debug)]
pub enum VmcClientError<E> {
    Comms(HostErr<WireError>),
    Endpoint(E),
}

impl<E> From<HostErr<WireError>> for VmcClientError<E> {
    fn from(value: HostErr<WireError>) -> Self {
        Self::Comms(value)
    }
}

pub struct VmcDriver {
    pub driver: HostClient<WireError>,
}

impl VmcDriver {
    pub fn new() -> Result<Self, String> {
        match HostClient::try_new_raw_nusb(
            |c| c.product_string() == Some("vmc"),
            ERROR_PATH,
            8,
            VarSeqKind::Seq2,
        ) {
            Ok(driver) => Ok(Self { driver }),
            Err(x) => Err(x),
        }
    }

    pub async fn item_available(&mut self, cmd: VendCommand) -> VendResult {
        match self.driver.send_resp::<ItemAvailable>(&cmd).await {
            Ok(result) => result,
            Err(_) => Err(VendError::CommsFault)
        }
    }

    pub async fn vend(&mut self, cmd: VendCommand) -> VendResult {
        match self.driver.send_resp::<Vend>(&cmd).await {
            Ok(result) => result,
            Err(_) => Err(VendError::CommsFault)
        }
    }

    pub async fn force_dispense(&mut self, cmd: VendCommand) -> VendResult {
        match self.driver.send_resp::<ForceDispense>(&cmd).await {
            Ok(result) => result,
            Err(_) => Err(VendError::CommsFault)
        }
    }
}
