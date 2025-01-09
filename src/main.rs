mod config;
mod spic_client;

use tokio;

use crate::spic_client::init_client;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // let config: Config = load_config().expect("Unable to load config file");
    // let datetime = Utc::now().to_rfc3339();



    let mut client = init_client();
    if client.authenticate().await? {
        println!("Authentication successful");
    } else {
        println!("Authentication failed");
        return Err("Authentication failed".into());
    }
    println!("Number of units: {}", client.number_of_units().await?);

    let unit_list = client.unit_list().await?;
    dbg!(&unit_list.first());
    Ok(())

}
