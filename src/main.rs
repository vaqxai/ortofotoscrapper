use futures::future::join_all;
use image::io::Reader as ImageReader;
use image::{GenericImage, ImageBuffer, ImageFormat, RgbImage};
use reqwest_middleware::ClientBuilder;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use std::io::Cursor;
use std::time::Instant;

const ADDR: &str = "https://ukosne.um.warszawa.pl/Data/2022/21";
const START_X: u128 = 1170859; // 1170859
const START_Y: u128 = 689916; // 689796
const SIZE_X: u128 = 40; // 2^2 = 4
const SIZE_Y: u128 = 40;
const CHUNKS_X: usize = 3;
const CHUNKS_Y: usize = 1;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    for cx in 0..CHUNKS_X {
        for cy in 0..CHUNKS_Y {
            let stx = START_X + (cx as u128 * 40);
            let sty = START_Y + (cy as u128 * 40);

            let start = Instant::now();
            println!("Starting chunk {} {} at {:.2?}", cx, cy, start.elapsed());
            let mut buf = RgbImage::new((SIZE_X * 256) as u32, (SIZE_Y * 256) as u32);

            let mut coords: Vec<(u32, u32)> = Vec::new();

            for x in stx..stx + SIZE_X {
                for y in sty..sty + SIZE_Y {
                    coords.push((x as u32, y as u32))
                }
            }

            let result: Vec<Result<(ImageBuffer<_, _>, u32, u32), Box<dyn std::error::Error>>> =
                join_all(coords.into_iter().map(|(x, y)| async move {
                    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
                    let client = ClientBuilder::new(reqwest::Client::new())
                        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
                        .build();

                    let result = client
                        .get(format!("{}/{}/{}.jpg", ADDR, x, y))
                        .send()
                        .await?
                        .bytes()
                        .await?;

                    let img = ImageReader::with_format(Cursor::new(result), ImageFormat::Jpeg)
                        .decode()?
                        .into_rgb8();

                    Ok((img, (x - stx as u32) * 256, (y - sty as u32) * 256))
                }))
                .await;

            println!("Compiling results of {} images", result.len());

            for img in result {
                match img {
                    Ok(img) => buf.copy_from(&img.0, img.1, img.2)?,
                    Err(e) => println!("Could not download: {:?}", e),
                };
            }

            println!("Saving image");

            buf.save(format!(
                "img\\chunk_[{} {}]({} {})-({} {}).jpg",
                cx,
                cy,
                stx,
                sty,
                stx + 40,
                sty + 40
            ))?;

            println!("Done, took {:.2?}", start.elapsed());
        }
    }

    Ok(())
}
