use openfare_lib::extension::FromLib;
use openfare_rs_lib;

fn main() {
    let env = env_logger::Env::new().filter_or("OPENFARE_RS_LOG", "off");
    env_logger::Builder::from_env(env).init();

    let mut extension = openfare_rs_lib::RsExtension::new();
    openfare_lib::extension::commands::run(&mut extension).unwrap();
}
