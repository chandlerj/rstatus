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
       
       let output = match CString::new(format!(" bat {:2} vol {:2} {}", battery, sys_volume, time)) {
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
   match timezone.parse(){
       Ok(tz) => {
            let utc: DateTime<Utc> = Utc::now();
            let local: DateTime<Tz> = utc.with_timezone(&tz);
            return local.format("%H:%M %m/%d").to_string()
       }
       Err(e) => {
            return e.to_string()
       }
   }
}


fn get_memory(sys: &mut sysinfo::System) -> String {
    sys.refresh_memory_specifics(MemoryRefreshKind::new().with_ram());

    let usage = ((sys.used_memory() as f64/sys.total_memory() as f64) * 100.0) as u64;


    let mut output = String::from("");
    if usage <= 50 {
        output.push_str(format!("^c#32a856^{}^d^", usage).as_str())
    }
    else if usage <= 60 {
        output.push_str(format!("^c#ebe134^{}^d^", usage).as_str())
    }    
    else if usage <= 70 {
        output.push_str(format!("^c#eb9534^{}^d^", usage).as_str())
    }
    else {
        output.push_str(format!("^c#eb3434^{}^d^", usage).as_str())
    }
    output


}


fn get_cpu_usage(sys: &mut sysinfo::System) -> String{
    sys.refresh_cpu_usage();
    let mut totalusage: f32 = 0.0;
    for cpu in sys.cpus(){
        totalusage += cpu.cpu_usage();
    } 
    let usage = (totalusage / sys.cpus().len() as f32) as u32;

    let mut output = String::from("");
    if usage <= 50 {
        output.push_str(format!("^c#32a856^{}^d^", usage).as_str())
    }
    else if usage <= 60 {
        output.push_str(format!("^c#ebe134^{}^d^", usage).as_str())
    }    
    else if usage <= 70 {
        output.push_str(format!("^c#eb9534^{}^d^", usage).as_str())
    }
    else {
        output.push_str(format!("^c#eb3434^{}^d^", usage).as_str())
    }
    output
}


fn get_battery_percentage() -> String {
    /*
     * Read the current battery percentage from the
     * class file 
     */

    let mut output = String::from("");

    match fs::read_to_string("/sys/class/power_supply/BAT0/capacity"){
        Ok(mut percent) => {
            percent.pop();

            let num_per:u16 = percent.parse().unwrap();

            if num_per <= 50 {
                output.push_str(format!("^c#eb3434^{}%^d^", num_per).as_str())
            }
            else if num_per <= 60 {
                output.push_str(format!("^c#eb9534^{}%^d^", num_per).as_str())
            }    
            else if num_per <= 70 {
                output.push_str(format!("^c#ebe134^{}%^d^", num_per).as_str())
            }
            else {
                output.push_str(format!("^c#32a856^{}%^d^", num_per).as_str())
            }
        }
        Err(_) => {
            eprintln!("Could not find battery");
            return String::from("err")
        }
    };

    match fs::read_to_string("/sys/class/power_supply/BAT0/status"){
        Ok(status) => {
            if status == "Charging\n" {
                output.push_str(" ^c#32a856^(crg)^d^")
            }
        }
        Err(_) => {
            eprintln!("Could not find charging status")
        }
    };



    output
}


fn get_system_volume() -> String {
    /*
     * Spawns a shell to execute the amixer command
     * and parses the output to determine the current
     * system volume while also checking if the channel is muted.
     */
    match Command::new("sh")
        .arg("-c")
        .arg("amixer sget Master | grep 'Front Right:' | awk -F'[][]' '{ print $2, $4 }'")
        .output()
    {
        Ok(output) => {
            let stdout = from_utf8(&output.stdout).unwrap().trim();
            let parts: Vec<&str> = stdout.split_whitespace().collect();

            if parts.len() == 2 {
                let volume = parts[0].trim_end_matches('%').parse::<u16>().unwrap_or(0);
                let is_muted = parts[1] == "off";

                if is_muted {
                    format!("^c#eb3434^{}%^d^ ^c#eb3434^(mut)^d^", volume) 
                } else {
                    format!("{}%", volume)
                }
            } else {
                eprintln!("Unexpected amixer output format.");
                String::from("")
            }
        }
        Err(_) => {
            eprintln!("Could not find audio device. Have you installed amixer?");
            String::from("")
        }
    }
}


fn get_system_temp(components: &mut Components) -> String {
    /*
     * Return the max temperature recorded on any sensor
     * @Input - A Component containing temp sensor data
     * @Output - the max temperature recorded on any sensor
     */

    components.refresh();
    let mut max_temp = 0;
    for component in components {
        let temp = component.temperature() as u16;
        if  temp > max_temp {
            max_temp = temp;
        }
    }
    
    let mut output = String::from("");
    if max_temp <= 50 {
        output.push_str(format!("^c#32a856^{}^d^", max_temp).as_str())
    }
    else if max_temp <= 60 {
        output.push_str(format!("^c#ebe134^{}^d^", max_temp).as_str())
    }    
    else if max_temp <= 70 {
        output.push_str(format!("^c#eb9534^{}^d^", max_temp).as_str())
    }
    else {
        output.push_str(format!("^c#eb3434^{}^d^", max_temp).as_str())
    }
    output
}
