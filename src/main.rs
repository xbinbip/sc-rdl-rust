mod rdl_config;
mod spic_client;
mod database;
mod logger_storage;

use rdl_config::init_config;
use spic_client::OnlineData;
use tokio;

use crate::spic_client::init_client;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {


    init_config()?;
    let config = rdl_config::CONFIG.read().unwrap();

    // let datetime = Utc::now().to_rfc3339();
    let test_unit_id = 82697;

    let db = database::Database::new().await?;

    db.init().await?;


    let mut client = init_client();
    if client.authenticate().await? {
        println!("Authentication successful");
    } else {
        println!("Authentication failed");
        return Err("Authentication failed".into());
    }
    println!("Number of units: {}", client.number_of_units().await?);

    let unit_list = client.unit_list().await?;

    let unit_ids = unit_list.iter().map(|unit| unit.id).collect::<Vec<i32>>();

    dbg!(&unit_list[12]);

    Ok(())

}
