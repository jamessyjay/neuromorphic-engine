use std::fs::File;
use std::io::{Read, Result, Error, ErrorKind};

pub fn load_mnist_images(path: &str) -> Result<Vec<Vec<f32>>> {
    let mut file = File::open(path)?;
    let mut magic_bytes = [0u8; 4];
    file.read_exact(&mut magic_bytes)?;
    let magic = u32::from_be_bytes(magic_bytes);
    if magic != 2051 {
        return Err(Error::new(ErrorKind::InvalidData, "Invalid magic number for MNIST images file"));
    }

    let mut num_images_bytes = [0u8; 4];
    file.read_exact(&mut num_images_bytes)?;
    let num_images = u32::from_be_bytes(num_images_bytes);

    let mut num_rows_bytes = [0u8; 4];
    file.read_exact(&mut num_rows_bytes)?;
    let num_rows = u32::from_be_bytes(num_rows_bytes);

    let mut num_cols_bytes = [0u8; 4];
    file.read_exact(&mut num_cols_bytes)?;
    let num_cols = u32::from_be_bytes(num_cols_bytes);

    let image_size = (num_rows * num_cols) as usize;
    let mut images = Vec::with_capacity(num_images as usize);

    for _ in 0..num_images {
        let mut image_bytes = vec![0u8; image_size];
        file.read_exact(&mut image_bytes)?;
        let image_f32: Vec<f32> = image_bytes.into_iter().map(|b| b as f32 / 255.0).collect();
        images.push(image_f32);
    }

    Ok(images)
}
