use magick_rust::magick_wand_genesis;
use std::fs::{DirBuilder, DirEntry};
use std::path::Path;
use std::process::Command;
use std::sync::{LazyLock, Once};
use std::env;
use std::fmt::format;
use magick_rust::CompressionType::Group4;
use tokio::task;

static START: Once = Once::new();
static GROUP_SIZE: u8 = 2;
static COMBINED_DIR: LazyLock<String> = LazyLock::new(|| {
    format!("combined-{:01}", GROUP_SIZE)
});

struct ImageMerge {
    img_vec: Vec<DirEntry>,
}

impl ImageMerge {
    fn add_image(&mut self, dir_entry: DirEntry) -> Result<(), &str> {
        if !self.is_full() {
            self.img_vec.push(dir_entry);
            return Ok(())
        }
        Err("Vec full")
    }

    fn is_full(&self) -> bool {
        self.img_vec.len() >= GROUP_SIZE as usize
    }

    fn is_empty(&self) -> bool {
        self.img_vec.is_empty()
    }

    fn process(&mut self, counter: u32) {
        let mut command = Command::new("magick");
        let ret = self.img_vec.iter().fold(String::with_capacity((GROUP_SIZE * 10) as usize), |acc_str, img| {
            let file_name = img.file_name().to_str().expect("Error").to_owned();
            //self.wand.read_image(file_name.as_str()).expect("TODO: panic message");
            command.arg(file_name.to_owned());
            acc_str + file_name.as_str() + " "
        });
        //fs::write("0.jpg", self.wand.evaluate_image(Mean, 0.0).unwrap()).expect("TODO: panic message");

        println!("Combining {:04}.jpg: {} ", counter, ret);
        command.arg("-evaluate-sequence").arg("Mean").arg(format!("{}/{:04}.jpg", COMBINED_DIR.as_str(), counter));

        command.output().expect("TODO: panic message");

    }

    fn reset(&mut self) {
        self.img_vec = vec![];
    }

}

#[tokio::main]
async fn main() {

    START.call_once(|| {
        magick_wand_genesis();
    });

    let mut threads: Vec<_> = vec![];
    let mut counter = 0;

    let path = env::args().skip(1).next().unwrap();
    let img_folder = Path::new(&path);
    env::set_current_dir(img_folder).expect("cd error");

    let iter = img_folder.read_dir().expect("read dir error").peekable();

    let mut image_merge = ImageMerge{
        img_vec: vec![],
    };
    
    if !std::fs::exists(COMBINED_DIR.as_str()).expect("TODO: panic message") {
        std::fs::create_dir(COMBINED_DIR.as_str()).expect("TODO: panic message");
    }

    for img in iter {
        if !image_merge.is_full() {
            image_merge.add_image(img.unwrap()).expect("Err");
        }

        if image_merge.is_full() {
            println!("Queuing job {:04}", counter);
            let handle = task::spawn(async move {
                image_merge.process(counter);
            });
            image_merge = ImageMerge{
                img_vec: vec![],
            };
            threads.push(handle);
            counter = counter + 1;
        }
        
        if counter >= 300 {
            break
        }
        
    }

    if !image_merge.is_empty() {
        let handle = task::spawn(async move {
            image_merge.process(counter);
        });
        threads.push(handle);
    }

    for handle in threads {
        handle.await.unwrap();
    }
    
}

