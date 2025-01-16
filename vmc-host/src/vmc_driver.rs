use postcard_rpc::{
    header::VarSeqKind,
    host_client::{HostClient, HostErr},
    standard_icd::{PingEndpoint, WireError, ERROR_PATH},
};

use vmc_icd::{Dispense, Dispenser, DispenserAddress, ForceDispense, GetDispenserInfo};

use std::convert::Infallible;

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
    pub fn new(devicename: &str) -> Result<Self, String> {
        match HostClient::try_new_raw_nusb(
            |c| c.product_string() == Some(devicename),
            ERROR_PATH,
            8,
            VarSeqKind::Seq2,
        ) {
            Ok(driver) => Ok(Self { driver }),
            Err(x) => Err(x),
        }
    }

    pub async fn ping(&mut self, seq: u32) -> bool {
        let val = self.driver.send_resp::<PingEndpoint>(&seq).await;
        match val {
            Ok(num) => {
                if num == seq {
                    true
                } else {
                    //Wrong sequence number....
                    false
                }
            }
            _ => false,
        }
    }

    pub async fn dispense(
        &mut self,
        addr: DispenserAddress,
    ) -> Result<(), VmcClientError<Infallible>> {
        let _res = self.driver.send_resp::<Dispense>(&addr).await?;
        Ok(())
    }

    pub async fn force_dispense(
        &mut self,
        addr: DispenserAddress,
    ) -> Result<(), VmcClientError<Infallible>> {
        let _res = self.driver.send_resp::<ForceDispense>(&addr).await?;
        Ok(())
    }

    pub async fn get_dispenser_info(&mut self, row: char, col: char)-> Option<Dispenser> {
        let addr = DispenserAddress { row, col };
        if let Ok(result) = self.driver.send_resp::<GetDispenserInfo>(&addr).await {
            result
        }
        else {
            None
        }
    }

    pub async fn map_machine(&mut self) -> Vec<Dispenser> {
        let mut dispensers: Vec<Dispenser> = Vec::new();
        //For all possible machine addresses, see if there is a dispenser present, and obtain its' status
        for row in ['A', 'B', 'C', 'D', 'E', 'F', 'G'] {
            for col in ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'] {
                let addr = DispenserAddress { row, col };
                if let Ok(result) = self.driver.send_resp::<GetDispenserInfo>(&addr).await {
                    match result {
                        Some(d) => {
                            dispensers.push(d);
                        }
                        None => {}
                    }
                }
            }
        }
        dispensers
    }
}
