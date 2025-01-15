use postcard_rpc::{
    header::VarSeqKind,
    host_client::{HostClient, HostErr},
    standard_icd::{PingEndpoint, WireError, ERROR_PATH},
};
use std::convert::Infallible;
use vmc_icd::{
    Dispense, DispenseError, DispenseResult, DispenserAddress, ForceDispense, GetDispenserInfo, DispenserOption
};

pub struct WorkbookClient {
    pub client: HostClient<WireError>,
}

#[derive(Debug)]
pub enum WorkbookError<E> {
    Comms(HostErr<WireError>),
    Endpoint(E),
}

impl<E> From<HostErr<WireError>> for WorkbookError<E> {
    fn from(value: HostErr<WireError>) -> Self {
        Self::Comms(value)
    }
}

trait FlattenErr {
    type Good;
    type Bad;
    fn flatten(self) -> Result<Self::Good, WorkbookError<Self::Bad>>;
}

impl<T, E> FlattenErr for Result<T, E> {
    type Good = T;
    type Bad = E;
    fn flatten(self) -> Result<Self::Good, WorkbookError<Self::Bad>> {
        self.map_err(WorkbookError::Endpoint)
    }
}

// ---

impl WorkbookClient {
    pub fn new() -> Self {
        let client = HostClient::new_raw_nusb(
            |d| d.product_string() == Some("vmc"),
            ERROR_PATH,
            8,
            VarSeqKind::Seq2,
        );
        Self { client }
    }

    pub async fn wait_closed(&self) {
        self.client.wait_closed().await;
    }

    pub async fn ping(&self, id: u32) -> Result<u32, WorkbookError<Infallible>> {
        let val = self.client.send_resp::<PingEndpoint>(&id).await?;
        Ok(val)
    }

    pub async fn dispense(&self, x: DispenserAddress) -> Result<DispenseResult, WorkbookError<Infallible>> {
        let result = self.client.send_resp::<Dispense>(&x).await?;
        Ok(result)
    }

    pub async fn force_dispense(&self, x: DispenserAddress) -> Result<DispenseResult, WorkbookError<Infallible>> {
        let result = self.client.send_resp::<ForceDispense>(&x).await?;
        Ok(result)
    }

    pub async fn dispenser_status(&self, x: DispenserAddress) -> Result<DispenserOption, WorkbookError<Infallible>> {
        let result = self.client.send_resp::<GetDispenserInfo>(&x).await?;
        Ok(result)
    }

}

impl Default for WorkbookClient {
    fn default() -> Self {
        Self::new()
    }
}
