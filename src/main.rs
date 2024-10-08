use sysinfo::{System, MemoryRefreshKind, Components};
use chrono::{Utc, DateTime};
use chrono_tz::Tz;
use std::ffi::CString;
use std::fs;
use std::time::Duration;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::rust_connection::RustConnection;
use std::process::Command;
use std::str::from_utf8;

fn main() {

   let mut sys = System::new_all(); 
   let (conn, screen_num) = RustConnection::connect(None).unwrap();
   let screen = &conn.setup().roots[screen_num];
   let wm_name_atom = conn.intern_atom(false, b"WM_NAME").unwrap().reply().unwrap().atom;
   let mut components = Components::new_with_refreshed_list();
   let utf8_string_atom = conn.intern_atom(false, b"UTF8_STRING").unwrap().reply().unwrap().atom;
   loop {
       let sys_temp = get_system_temp(&mut components);
       let cpu_usage = get_cpu_usage(&mut sys);
       let memory = get_memory(&mut sys);
       let time = get_time(&mut "America/Denver");
       let battery = get_battery_percentage();
       let sys_volume = get_system_volume();
       
       let output = match CString::new(format!(" vol: {:2}%  temp: {:2}C  bat: {:2}%  cpu: {:2}%  mem: {:2}%  {}", sys_volume, sys_temp, battery, cpu_usage, memory, time)) {
            Ok(out) => out,
            Err(e) => {

               eprintln!("{}", e.to_string());
               continue;
            }
       };

       let out = output.as_bytes();
       conn.change_property(
           PropMode::REPLACE,
           screen.root,
           wm_name_atom,
           utf8_string_atom,
           8,
           out.len() as u32,
           out,
       ).unwrap();

       conn.flush().unwrap();

       std::thread::sleep(Duration::new(1, 0));
   }


}

fn get_time(timezone: &str) -> String {
   let tz = timezone.parse(); 
   match tz{

       Ok(tz) => {
            let utc: DateTime<Utc> = Utc::now();
            let local: DateTime<Tz> = utc.with_timezone(&tz);
            local.format("%H:%M:%S %m/%d").to_string()
       }
       Err(e) => {
            e.to_string()
       }
   }
}


fn get_memory(sys: &mut sysinfo::System) -> u64 {
    sys.refresh_memory_specifics(MemoryRefreshKind::new().with_ram());
    if sys.total_memory() == 0 {
        return 0
    }
    ((sys.used_memory() as f64/sys.total_memory() as f64) * 100.0) as u64
}


fn get_cpu_usage(sys: &mut sysinfo::System) -> u32{
    sys.refresh_cpu_usage();
    let mut totalusage: f32 = 0.0;
    for cpu in sys.cpus(){
        totalusage += cpu.cpu_usage();
    } 
    (totalusage / sys.cpus().len() as f32) as u32
}

fn get_battery_percentage() -> String {
    let mut output = String::from("");

    match fs::read_to_string("/sys/class/power_supply/BAT0/status"){
        Ok(status) => {
            if status == "Charging\n" {
                output.push_str("(crg) ")
            }
        }
        Err(_) => {
            eprintln!("Could not find charging status")
        }
    };

    match fs::read_to_string("/sys/class/power_supply/BAT0/capacity"){
        Ok(mut percent) => {
            percent.pop();
            output.push_str(&percent);
        }
        Err(_) => {
            eprintln!("Could not find battery");
            return String::from("err")
        }
    };
    output
}

fn get_system_volume() -> u16 {
    match Command::new("sh")
        .arg("-c")
        .arg("amixer sget Master | grep 'Right:' | awk -F'[][]' '{ print $2 }' | sed 's/%//'")
        .output(){
            Ok(output) => {
                let stdout = from_utf8(&output.stdout).unwrap().trim();
                stdout.parse::<u16>().unwrap()
            },


            Err(_) => {
                eprintln!("could not find audio device. have you installed amixer?");
                0
            }

        }

}

fn get_system_temp(components: &mut Components) -> u16 {
    components.refresh();
    let mut max_temp = 0;
    for component in components {
        let temp = component.temperature() as u16;
        if  temp > max_temp {
            max_temp = temp;
        }
    }
    max_temp
}
