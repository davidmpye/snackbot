use postcard_rpc::{
    header::VarSeqKind,
    host_client::{HostClient, HostErr},
    standard_icd::{PingEndpoint, WireError, ERROR_PATH},
};

use keyboard_icd::{SetBacklight, SetText};

use std::convert::Infallible;

#[derive(Debug)]
pub enum ClientError<E> {
    Comms(HostErr<WireError>),
    Endpoint(E),
}

impl<E> From<HostErr<WireError>> for ClientError<E> {
    fn from(value: HostErr<WireError>) -> Self {
        Self::Comms(value)
    }
}

pub struct LcdDriver {
    pub driver: HostClient<WireError>,
}

impl LcdDriver {
    pub fn new() -> Result<Self, String> {
        match HostClient::try_new_raw_nusb(
            |c| c.product_string() == Some("keyboard"),
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

    pub async fn set_backlight(&mut self, on: bool) -> Result<(), ClientError<Infallible>> {
        let _res = self.driver.send_resp::<SetBacklight>(&on).await?;
        Ok(())
    }

    pub async fn set_text(
        &mut self,
        line1: String,
        line2: String,
    ) -> Result<(), ClientError<Infallible>> {
        let mut l1_copy = line1;
        l1_copy.truncate(32);
        let mut l2_copy = line2;
        l2_copy.truncate(32);

        //Trailing whitespace stripped off at remote end
        let l1: [u8; 32] = format!("{: <32}", l1_copy).as_bytes().try_into().unwrap();
        let l2: [u8; 32] = format!("{: <32}", l2_copy).as_bytes().try_into().unwrap();
        let _res = self.driver.send_resp::<SetText>(&([l1, l2])).await?;
        Ok(())
    }
}
