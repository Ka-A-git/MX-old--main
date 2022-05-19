use ctrlc;
use mx::Logger;
use mx::Platform;
use std::io::stdin;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), &'static str> {
    Logger::init();

    let platform: &'static Platform = Box::leak(Box::new(Platform::init()));

    platform.start()?;

    // let gateways_lock = Box::leak(Box::new(
    //     platform
    //         .environment
    //         .gateway_environment
    //         .gateways
    //         .read()
    //         .unwrap(),
    // ));
    // // for gateway in gateways_lock.iter() {
    //     gateway.start()?;
    // }

    // let robots_lock = Box::leak(Box::new(
    //     platform
    //         .environment
    //         .robot_environment
    //         .robots
    //         .read()
    //         .unwrap(),
    // ));
    // for robot in robots_lock.iter() {
    //     robot.start()?;
    // }

    // thread::sleep(Duration::from_secs(10));

    // let gateway1 = gateways_lock.first().unwrap();
    // gateway1.stop()?;

    // let robot1 = robots_lock.first().unwrap();
    // robot1.stop()?;

    // thread::sleep(Duration::from_secs(30));

    ctrlc::set_handler(move || {
        info!("Stopping platform with Ctrl+C signal");

        platform.stop().unwrap();

        // We stop the current process so as not to call a second time platform.stop() function
        std::process::exit(0);
    })
    .expect("Error Ctrl-C handler");

    let mut input = String::new();
    stdin().read_line(&mut input).expect("Not correct input");

    platform.stop()?;

    Ok(())
}
