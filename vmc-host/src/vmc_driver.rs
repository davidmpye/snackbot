use postcard_rpc::{
    header::VarSeqKind,
    host_client::{HostClient, HostErr},
    standard_icd::{PingEndpoint, WireError, ERROR_PATH},
};

use vmc_icd::{dispenser::{ Dispenser, DispenserAddress}, SetCoinAcceptorEnabled};
use vmc_icd::{Dispense, ForceDispense, GetDispenserInfo};
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

    pub async fn dispense(&mut self, addr: DispenserAddress) -> Result<(), VmcClientError<Infallible>>{
        let _res = self.driver.send_resp::<Dispense>(&addr).await?;
        Ok(())
    }

    pub async fn force_dispense(&mut self, addr: DispenserAddress) -> Result<(), VmcClientError<Infallible>>{
        let _res = self.driver.send_resp::<ForceDispense>(&addr).await?;
        Ok(())
    }

    pub async fn map_machine(&mut self) -> Vec<Dispenser> { 
        let dispensers:Vec<Dispenser> = Vec::new();
        //For all possible machine addresses, see if there is a dispenser present
        for r in [ 'A', 'B', 'C', 'D', 'E', 'F','G' ] {
            for c in ['0','1','2','3','4','5','6','7','8','9'] {
                let disp = self.driver.send_resp::<GetDispenserInfo>(&DispenserAddress{row:r, col:c}).await;
            }
        }
        dispensers
    }

    //Sets whether the coin acceptor should accept coins or not
    pub async fn set_coinacceptor_enabled(&mut self, enable:bool) -> Result<(), VmcClientError<Infallible>> {
        let _res = self.driver.send_resp::<SetCoinAcceptorEnabled>(&enable).await?;
        Ok(())
    }

    pub async fn dispense_coins(&mut self, value: u16) -> Result<(), ()> {

        Ok(())
    }



}
