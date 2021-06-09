use anyhow::Result;
/// Welcome to the test.
use std::{env, fs::File};

pub mod core;

#[tokio::main]
async fn main() -> Result<()> {
    let task = core::Task::new();

    // Get arguments, skip executable path.
    let mut args = env::args_os();
    args.next();

    let transactions_filename = args
        .next()
        .ok_or(anyhow::anyhow!("File name not provided"))?;
    let mut reader = File::open(transactions_filename).map(csv::Reader::from_reader)?;

    // Send our transactions to the transaction processor. Made this way
    // So we can send more transactions from other tasks if necessary.
    for record in reader.deserialize::<core::RawTransaction>() {
        if let Ok(tx) = record {
            task.send_tx(tx)?;
        }
    }

    // Lets close and get the results.
    let accounts = task.close()?;
    let mut writer = csv::Writer::from_writer(std::io::stdout());
    for acc in accounts.data {
        let raw_acc: core::RawAccount = acc.1.into();
        writer.serialize(raw_acc)?;
    }

    Ok(())
}
