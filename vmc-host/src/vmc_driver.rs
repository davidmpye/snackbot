use postcard_rpc::{
    header::VarSeqKind,
    host_client::{HostClient, HostErr},
    standard_icd::{PingEndpoint, WireError, ERROR_PATH},
};

use vmc_icd::{cashless_device::CashlessDeviceCommand, dispenser::{ DispenseCommand, Dispenser, DispenserAddress}, CashlessDeviceCmdEndpoint, DispenserStatusEndpoint };//; SetCoinAcceptorEnabled};
use vmc_icd::{CoinAcceptorEnableEndpoint,DispenseEndpoint};
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

use vmc_icd::coin_acceptor::{CoinAcceptorEvent, CoinInserted, CoinRouting,};

use vmc_icd::cashless_device::CashlessDeviceCommand::*;

#[derive (Copy, Clone)]
pub enum VmcCommand {
    VendItem(char,char),
    ForceVendItem(char, char),
    GetMachineMap(),                //Get a vec of dispenser
    GetDispenser(char,char),            //Get information about a specific dispenser
    SetCoinAcceptorEnabled(bool),   //Whether the coin acceptor should accept coins
    RefundCoins(u16),               //Refund amount
    CashlessCmd(CashlessDeviceCommand) //
}

pub enum VmcResponse {
    MachineMap(Vec<Dispenser>),
    Dispenser(Dispenser),
    //Vend result for a vend request
    CoinAcceptorEvent(CoinAcceptorEvent),
    CoinInsertedEvent(CoinInserted)
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
        let _res = self.driver.send_resp::<DispenseEndpoint>(&DispenseCommand::Vend(addr)).await?;
        Ok(())
    }

    pub async fn force_dispense(&mut self, addr: DispenserAddress) -> Result<(), VmcClientError<Infallible>>{
        let _res = self.driver.send_resp::<DispenseEndpoint>(&DispenseCommand::ForceVend(addr)).await?;
        Ok(())
    }

    pub async fn map_machine(&mut self) -> Vec<Dispenser> { 
        let dispensers:Vec<Dispenser> = Vec::new();
        //For all possible machine addresses, see if there is a dispenser present
        for r in [ 'A', 'B', 'C', 'D', 'E', 'F','G' ] {
            for c in ['0','1','2','3','4','5','6','7','8','9'] {
                let disp = self.driver.send_resp::<DispenserStatusEndpoint>(&DispenserAddress{row:r, col:c}).await;
            }
        }
        dispensers
    }

    //Sets whether the coin acceptor should accept coins or not
    pub async fn set_coinacceptor_enabled(&mut self, enable:bool) -> Result<(), VmcClientError<Infallible>> {
        let _res = self.driver.send_resp::<CoinAcceptorEnableEndpoint>(&enable).await?;
        Ok(())
    }

    pub async fn dispense_coins(&mut self, value: u16) -> Result<(u16), VmcClientError<Infallible>> {
      //  let amount_refunded = self.driver.send_resp::<DispenseCoins>(&value).await?;
        Ok(10)
    }

    pub async fn send_cashless_device_command(&mut self, cmd: CashlessDeviceCommand) -> Result<(),VmcClientError<Infallible>> {
       let res  =self.driver.send_resp::<CashlessDeviceCmdEndpoint>(&cmd).await?;
        //Fixme
        Ok(())
    }
}
