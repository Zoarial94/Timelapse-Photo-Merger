use magick_rust::{magick_wand_genesis};
use std::path::Path;
use std::process::Command;
use std::sync::{LazyLock, Once};
use std::env;
use circular_buffer::CircularBuffer;
use tokio::task;

static START: Once = Once::new();
static GROUP_SIZE: u8 = 3;
static COMBINED_DIR: LazyLock<String> = LazyLock::new(|| {
    format!("combined-{:01}", GROUP_SIZE)
});

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

        println!("Combining {:04}.jpg: {} ", counter, ret);
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
    let mut img_buffer : CircularBuffer<{ GROUP_SIZE as usize }, String> = CircularBuffer::new();

    let path = env::args().skip(1).next().unwrap();
    let img_folder = Path::new(&path);
    env::set_current_dir(img_folder).expect("cd error");

    let iter = img_folder.read_dir().expect("read dir error").peekable();


    if !std::fs::exists(COMBINED_DIR.as_str()).expect("TODO: panic message") {
        std::fs::create_dir(COMBINED_DIR.as_str()).expect("TODO: panic message");
    }

    for img in iter {
        
        let file_name = img.unwrap().file_name().into_string().unwrap();
        img_buffer.push_back(file_name);
        
        let mut image_merge = ImageMerge {
            img_vec: img_buffer.iter().cloned().collect(), 
        };
        
        println!("Queuing job {:04}", counter);
        let handle = task::spawn(async move {
            image_merge.process(counter);
        });
        threads.push(handle);
        counter = counter + 1;
        
        // if counter >= 300 {
        //     break
        // }
        
    }

    // for handle in threads {
    //     handle.await.unwrap();
    // }
    
}

