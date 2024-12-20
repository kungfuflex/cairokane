use alkanes_runtime::{token::Token};
#[allow(unused_imports)]
use alkanes_runtime::{
    println,
    stdio::{stdout, Write},
};
use anyhow::{anyhow, Result};
use alkanes_runtime::{runtime::AlkaneResponder, storage::StoragePointer};
use alkanes_support::{witness::find_witness_payload, utils::{shift_or_err}};
use alkanes_support::{context::Context, parcel::{AlkaneTransfer, AlkaneTransferParcel}, response::CallResponse};
use metashrew_support::{utils::{consensus_decode}, compat::{to_arraybuffer_layout, to_passback_ptr}};
use metashrew_support::index_pointer::KeyValuePointer;
use protorune_support::{protostone::{Protostone}};
use ordinals::{Runestone, Artifact};
use bitcoin::{Transaction};
use std::sync::Arc;

#[derive(Default)]
pub struct Cairokane(());

pub trait MintableToken: Token {
    fn mint(&self, context: &Context, value: u128) -> AlkaneTransfer {
        AlkaneTransfer {
            id: context.myself.clone(),
            value,
        }
    }
}

impl Token for Cairokane {
    fn name(&self) -> String {
        String::from("CAIROKANE PEG")
    }
    fn symbol(&self) -> String {
        String::from("CAIRO")
    }
}
impl MintableToken for Cairokane {}

impl Cairokane {
  fn public_pointer(&self) -> StoragePointer {
    StoragePointer::from_keyword("/public")
  }
  #[allow(dead_code)]
  fn public(&self) -> Vec<u8> {
    self.public_pointer().get().as_ref().clone()
  }
  fn set_public(&self, context: &Context, _vout: u32) -> Result<()> {
    let vout = _vout as usize;
    let tx = consensus_decode::<Transaction>(&mut std::io::Cursor::new(self.transaction()))?;
    if let Some(Artifact::Runestone(ref runestone)) = Runestone::decipher(&tx) {
      let protostones = Protostone::from_runestone(runestone)?;
      let message = &protostones[(context.vout as usize) - tx.output.len() - 1];
      if message.edicts.len() != 0 {
        return Err(anyhow!("message cannot contain edicts, only a pointer"));
      }
      let pointer = message
        .pointer
        .ok_or("")
        .map_err(|_| anyhow!("no pointer in message"))?;
      if pointer as usize >= tx.output.len() {
        return Err(anyhow!("pointer cannot be a protomessage"));
      }
      if pointer as usize == vout {
        return Err(anyhow!("pointer cannot be equal to output spendable by synthetic"));
      }
      self.public_pointer().set(Arc::new(tx.output[vout as usize].script_pubkey.as_bytes()[2..34].to_vec()));
      Ok(())
    } else {
      Err(anyhow!("unexpected condition: execution occurred with no Protostone present"))
    }
  }
  fn parcel_pointer(&self) -> StoragePointer {
    StoragePointer::from_keyword("/parcel")
  }
  fn store_parcel(&self, v: &AlkaneTransferParcel) {
    self.parcel_pointer().set(Arc::new(v.serialize()));
  }
  fn parcel(&self) -> Result<AlkaneTransferParcel> {
    AlkaneTransferParcel::parse(&mut std::io::Cursor::new(self.parcel_pointer().get().as_ref().clone()))
  }
  fn cairo_run(&self, _proof: Vec<u8>) -> Result<()> {
    Ok(())
  }
  fn verify_proof(&self) -> Result<()> {
    let tx = consensus_decode::<Transaction>(&mut std::io::Cursor::new(self.transaction()))?;
    let proof = find_witness_payload(&tx, 0)
                    .ok_or("")
                    .map_err(|_| anyhow!("witness envelope at index 0 does not contain data"))?;
     
   // let pk = self.public();
    self.cairo_run(proof)?;
    Ok(())
  }
}

impl AlkaneResponder for Cairokane {
    fn execute(&self) -> Result<CallResponse> {
        let context = self.context()?;
        let mut inputs = context.inputs.clone();
        let mut response: CallResponse = CallResponse::default();
        match shift_or_err(&mut inputs)? {
            /* initialize(u128, u128) */
            0 => {
                let mut pointer = StoragePointer::from_keyword("/initialized");
                if pointer.get().len() == 0 {
                    self.store_parcel(&context.incoming_alkanes);
                    self.set_public(&context, shift_or_err(&mut inputs)?.try_into()?)?;
                    pointer.set(Arc::new(vec![0x01]));
                    Ok(response)
                } else {
                    Err(anyhow!("already initialized"))
                }
            },
            78 => {
                self.verify_proof()?;
                response.alkanes = self.parcel()?;
                Ok(response)
            }
            /* name() */
            99 => {
                response.data = self.name().into_bytes().to_vec();
                Ok(response)
            }
            /* symbol() */
            100 => {
                response.data = self.symbol().into_bytes().to_vec();
                Ok(response)
            }
            _ => {
                Err(anyhow!("unrecognized opcode"))
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn __execute() -> i32 {
    let mut response = to_arraybuffer_layout(&Cairokane::default().run());
    to_passback_ptr(&mut response)
}
