use sysinfo::{System, MemoryRefreshKind};
use chrono::{Utc, DateTime};
use chrono_tz::Tz;
use std::ffi::CString;
use std::fs;
use std::time::Duration;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::rust_connection::RustConnection;



fn main() {

   let mut sys = System::new_all(); 
   let (conn, screen_num) = RustConnection::connect(None).unwrap();
   let screen = &conn.setup().roots[screen_num];
   let wm_name_atom = conn.intern_atom(false, b"WM_NAME").unwrap().reply().unwrap().atom;
   let utf8_string_atom = conn.intern_atom(false, b"UTF8_STRING").unwrap().reply().unwrap().atom;

   loop {

       let bat_state = get_battery_percentage();
       let cpu_usage = get_cpu_usage(&mut sys);
       let memory = get_memory(&mut sys);
       let time = get_time(&mut "America/Denver");
       
       let output = match CString::new(format!("bat: {:2}%  cpu: {:2}%  mem: {:2}%  {}", bat_state, cpu_usage, memory, time)) {

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

fn get_battery_percentage() -> i32 {
    let percentage = match fs::read_to_string("/sys/class/power_supply/BAT0/capacity"){
        Ok(mut percent) => {
            percent.pop();
            let as_int: i32 = percent.parse().unwrap();
            as_int
        }
        Err(_) => {
            eprintln!("Could not find battery");
            return 0
        }
    };

    percentage
}
