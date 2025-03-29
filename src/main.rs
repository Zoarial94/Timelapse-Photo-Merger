use circular_buffer::CircularBuffer;
use magick_rust::magick_wand_genesis;
use std::env;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering::Release;
use std::sync::{Arc, LazyLock, Once};
use tokio::sync::Notify;
use tokio::task;

static START: Once = Once::new();
static GROUP_SIZE: usize = 3;
static COMBINED_DIR: LazyLock<String> = LazyLock::new(|| {
    format!("combined-{:01}", GROUP_SIZE)
});

static MAX_OPERATIONS: usize = 24; // This should be 1.5 the number of cpu cores for max utilization (cpu cores, not threads)

struct ImageMerge {
    img_vec: Vec<String>,
}

impl ImageMerge {
    fn process(&mut self, counter: u32) {
        let mut command = Command::new("magick");
        let ret = self.img_vec.iter().fold(String::with_capacity((GROUP_SIZE * 10) as usize), |acc_str, img| {
            let file_name = img;
            //self.wand.read_image(file_name.as_str()).expect("TODO: panic message");
            command.arg(file_name.to_owned());
            acc_str + file_name + " "
        });
        //fs::write("0.jpg", self.wand.evaluate_image(Mean, 0.0).unwrap()).expect("TODO: panic message");

        println!("({counter}) Combining {:04}.jpg: {} ", counter, ret);
        command.arg("-evaluate-sequence").arg("Mean").arg(format!("{}/{:04}.jpg", COMBINED_DIR.as_str(), counter));

        command.output().expect("TODO: panic message");

    }
}

#[tokio::main]
async fn main() {

    START.call_once(|| {
        magick_wand_genesis();
    });

    let mut threads: Vec<_> = vec![];
    let mut counter = 0;
    let mut img_buffer : CircularBuffer<{ GROUP_SIZE }, String> = CircularBuffer::new();

    let path = env::args().skip(1).next().unwrap();
    let img_folder = Path::new(&path);
    env::set_current_dir(img_folder).expect("cd error");
    let images_list: Vec<_> = img_folder.read_dir().expect("read dir error").filter(|item| {
        item.as_ref().unwrap().file_type().unwrap().is_file()
    }).collect();
    let iter = images_list.iter().peekable();

    if !std::fs::exists(COMBINED_DIR.as_str()).expect("TODO: panic message") {
        std::fs::create_dir(COMBINED_DIR.as_str()).expect("TODO: panic message");
    }

    let notify = Arc::new(Notify::new());

    let operations_total = images_list.len();
    let operations_complete = Arc::new(AtomicU32::new(0));


    for img in iter {
        
        let file_name = img.as_ref().unwrap().file_name().into_string().unwrap();
        img_buffer.push_back(file_name);
        
        let mut image_merge = ImageMerge {
            img_vec: img_buffer.iter().cloned().collect(), 
        };

        let notify2 = notify.clone();
        let complete2 = operations_complete.clone();
        let handle = task::spawn_blocking( move || {
            image_merge.process(counter);
            let completed = 1 + complete2.fetch_add(1, Release) as usize;
            println!("{:04.1}% complete ({completed}/{operations_total})", completed as f64 / operations_total as f64 * 100.0);
            notify2.notify_one();
        });
        threads.push(handle);
        counter = counter + 1;

        if threads.len() >= MAX_OPERATIONS {
            notify.notified().await;
            threads = threads.into_iter().filter( |t| {
                !t.is_finished()
            }).collect();
        }
        
        // if counter >= 300 {
        //     break
        // }

    }
    
    for handle in threads {
        handle.await.unwrap();
    }
    
    
}

