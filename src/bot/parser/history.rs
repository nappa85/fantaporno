use std::io::Cursor;

use plotters::{
    backend::{PixelFormat, RGBPixel},
    prelude::*,
};
use sea_orm::{ConnectionTrait, StreamTrait};
use tgbot::{
    api::Client,
    types::{
        mime, InputFile, InputFileReader, ParseMode, ReplyParameters, SendMessage, SendPhoto, User,
    },
};

use crate::Error;

use super::{Chat, Lang};

const IMAGE_WIDTH: u32 = 640;
const IMAGE_HEIGHT: u32 = 480;

pub async fn execute<C>(
    client: &Client,
    conn: &C,
    user: &User,
    message_id: i64,
    chat: &Chat,
    pornstar_name: String,
) -> Result<Result<(), String>, Error>
where
    C: ConnectionTrait + StreamTrait,
{
    let player = match crate::entities::player::find(conn, user, chat.id, chat.lang).await? {
        Ok(player) => player,
        Err(err) => return Ok(Err(err)),
    };

    let pornstar = match crate::entities::pornstar::search(conn, &pornstar_name, chat.lang).await? {
        Ok(pornstar) => pornstar,
        Err(err) => return Ok(Err(err)),
    };

    let mut history = player.history(conn, Some([pornstar.id])).await?;

    if let Some(positions) = history.remove(&pornstar.id) {
        let history = positions.scores().rev().take(20).collect::<Vec<_>>();

        let mut buf = vec![0; IMAGE_WIDTH as usize * IMAGE_HEIGHT as usize * RGBPixel::PIXEL_SIZE];
        let root =
            BitMapBackend::with_buffer(&mut buf, (IMAGE_WIDTH, IMAGE_HEIGHT)).into_drawing_area();
        root.fill(&WHITE)
            .map_err(|err| Error::Plotter(Box::new(err)))?;
        let mut chart = ChartBuilder::on(&root)
            .caption(
                match chat.lang {
                    Lang::En => format!(
                        "Pornstar \"{}\" last {} contributions:",
                        pornstar.name,
                        history.len()
                    ),
                    Lang::It => format!(
                        "Ultimi {} punteggi del/della pornostar \"{}\":",
                        history.len(),
                        pornstar.name
                    ),
                },
                ("sans-serif", 20).into_font(),
            )
            .margin(5)
            .x_label_area_size(30)
            .y_label_area_size(30)
            .build_cartesian_2d(
                history
                    .last()
                    .map(|(date, _points)| date.date())
                    .unwrap_or_default()
                    ..history
                        .first()
                        .map(|(date, _points)| date.date())
                        .unwrap_or_default(),
                history
                    .iter()
                    .map(|(_date, points)| *points)
                    .min()
                    .unwrap_or_default()
                    .min(0)
                    ..history
                        .iter()
                        .map(|(_date, points)| *points)
                        .max()
                        .unwrap_or_default(),
            )
            .map_err(|err| Error::Plotter(Box::new(err)))?;

        chart
            .configure_mesh()
            .draw()
            .map_err(|err| Error::Plotter(Box::new(err)))?;

        chart
            .draw_series(LineSeries::new(
                history
                    .into_iter()
                    .rev()
                    .map(|(date, points)| (date.date(), points)),
                &RED,
            ))
            .map_err(|err| Error::Plotter(Box::new(err)))?
            .label(pornstar.name.as_str())
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));

        chart
            .configure_series_labels()
            .background_style(WHITE.mix(0.8))
            .border_style(BLACK)
            .draw()
            .map_err(|err| Error::Plotter(Box::new(err)))?;

        root.present()
            .map_err(|err| Error::Plotter(Box::new(err)))?;
        drop(chart);
        drop(root);

        let img = image::DynamicImage::ImageRgb8(
            image::RgbImage::from_vec(IMAGE_WIDTH, IMAGE_HEIGHT, buf)
                .ok_or_else(|| Error::Plotter(Box::new(BufferNotBigEnough)))?,
        );
        let mut buf = Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png)
            .map_err(|err| Error::Plotter(Box::new(err)))?;
        buf.set_position(0);

        client
            .execute(
                SendPhoto::new(
                    chat.id,
                    InputFile::Reader(
                        InputFileReader::new(buf)
                            .with_file_name("history.png")
                            .with_mime_type(mime::IMAGE_PNG),
                    ),
                )
                .with_reply_parameters(ReplyParameters::new(message_id))
                .map_err(|err| Error::Plotter(Box::new(err)))?,
            )
            .await?;
    } else {
        let msg = match chat.lang {
            Lang::En => {
                format!("Pornstar \"{}\" never made points for you", pornstar.link())
            }
            Lang::It => format!(
                "Il/la pornostar \"{}\" non ha mai generato punti per te",
                pornstar.link()
            ),
        };

        client
            .execute(
                SendMessage::new(chat.id, msg)
                    .with_parse_mode(ParseMode::Markdown)
                    .with_reply_parameters(ReplyParameters::new(message_id)),
            )
            .await?;
    }

    Ok(Ok(()))
}

#[derive(Debug, thiserror::Error)]
#[error("Buffer not big enough")]
pub struct BufferNotBigEnough;
