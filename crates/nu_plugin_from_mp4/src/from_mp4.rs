use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, ReturnValue, TaggedDictBuilder, UntaggedValue, Value};
use nu_source::Tag;
use std::fs::File;
use std::io::Write;
use std::path::Path;

#[derive(Default)]
pub struct FromMp4 {
    pub state: Vec<u8>,
    pub name_tag: Tag,
}

impl FromMp4 {
    pub fn new() -> Self {
        Self {
            state: vec![],
            name_tag: Tag::unknown(),
        }
    }
}

pub fn convert_mp4_file_to_nu_value(path: &Path, tag: Tag) -> Result<Value, mp4::Error> {
    let mp4 = mp4::read_mp4(File::open(path).expect("Could not open mp4 file to read metadata"))?;

    let mut dict = TaggedDictBuilder::new(tag.clone());

    // Build tracks table
    let mut tracks = Vec::new();
    for track in mp4.tracks().values() {
        let mut curr_track_dict = TaggedDictBuilder::new(tag.clone());

        curr_track_dict.insert_untagged("track id", UntaggedValue::int(track.track_id()));

        curr_track_dict.insert_untagged(
            "track type",
            match track.track_type() {
                Ok(t) => UntaggedValue::string(t.to_string()),
                Err(_) => UntaggedValue::from("Unknown"),
            },
        );

        curr_track_dict.insert_untagged(
            "media type",
            match track.media_type() {
                Ok(t) => UntaggedValue::string(t.to_string()),
                Err(_) => UntaggedValue::from("Unknown"),
            },
        );

        curr_track_dict.insert_untagged(
            "box type",
            match track.box_type() {
                Ok(t) => UntaggedValue::string(t.to_string()),
                Err(_) => UntaggedValue::from("Unknown"),
            },
        );

        curr_track_dict.insert_untagged("width", UntaggedValue::int(track.width()));
        curr_track_dict.insert_untagged("height", UntaggedValue::int(track.height()));
        curr_track_dict.insert_untagged("frame_rate", UntaggedValue::from(track.frame_rate()));

        curr_track_dict.insert_untagged(
            "sample freq index",
            match track.sample_freq_index() {
                Ok(sfi) => UntaggedValue::string(sfi.freq().to_string()), // this is a string for formatting reasons
                Err(_) => UntaggedValue::from("Unknown"),
            },
        );

        curr_track_dict.insert_untagged(
            "channel config",
            match track.channel_config() {
                Ok(cc) => UntaggedValue::string(cc.to_string()),
                Err(_) => UntaggedValue::from("Unknown"),
            },
        );

        curr_track_dict.insert_untagged("language", UntaggedValue::string(track.language()));
        curr_track_dict.insert_untagged("timescale", UntaggedValue::int(track.timescale()));
        curr_track_dict.insert_untagged(
            "duration",
            UntaggedValue::duration(track.duration().as_nanos()),
        );
        curr_track_dict.insert_untagged("bitrate", UntaggedValue::int(track.bitrate()));
        curr_track_dict.insert_untagged("sample count", UntaggedValue::int(track.sample_count()));

        curr_track_dict.insert_untagged(
            "video profile",
            match track.video_profile() {
                Ok(vp) => UntaggedValue::string(vp.to_string()),
                Err(_) => UntaggedValue::from("Unknown"),
            },
        );

        curr_track_dict.insert_untagged(
            "audio profile",
            match track.audio_profile() {
                Ok(ap) => UntaggedValue::string(ap.to_string()),
                Err(_) => UntaggedValue::from("Unknown"),
            },
        );

        curr_track_dict.insert_untagged(
            "sequence parameter set",
            match track.sequence_parameter_set() {
                Ok(sps) => UntaggedValue::string(format!("{:X?}", sps)),
                Err(_) => UntaggedValue::from("Unknown"),
            },
        );

        curr_track_dict.insert_untagged(
            "picture parameter set",
            match track.picture_parameter_set() {
                Ok(pps) => UntaggedValue::string(format!("{:X?}", pps)),
                Err(_) => UntaggedValue::from("Unknown"),
            },
        );

        // push curr track to tracks vec
        tracks.push(curr_track_dict.into_value());
    }

    dict.insert_untagged("size", UntaggedValue::big_int(mp4.size()));

    dict.insert_untagged(
        "major brand",
        UntaggedValue::string(mp4.major_brand().to_string()),
    );

    dict.insert_untagged("minor version", UntaggedValue::int(mp4.minor_version()));

    dict.insert_untagged(
        "compatible brands",
        UntaggedValue::string(format!("{:?}", mp4.compatible_brands())),
    );

    dict.insert_untagged(
        "duration",
        UntaggedValue::duration(mp4.duration().as_nanos()),
    );

    dict.insert_untagged("timescale", UntaggedValue::int(mp4.timescale()));
    dict.insert_untagged("is fragmented", UntaggedValue::boolean(mp4.is_fragmented()));
    dict.insert_untagged("tracks", UntaggedValue::Table(tracks).into_value(&tag));

    Ok(dict.into_value())
}

pub fn from_mp4_bytes_to_value(mut bytes: Vec<u8>, tag: Tag) -> Result<Value, std::io::Error> {
    let mut tempfile = tempfile::NamedTempFile::new()?;
    tempfile.write_all(bytes.as_mut_slice())?;
    match convert_mp4_file_to_nu_value(tempfile.path(), tag) {
        Ok(value) => Ok(value),
        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
    }
}

pub fn from_mp4(bytes: Vec<u8>, name_tag: Tag) -> Result<Vec<ReturnValue>, ShellError> {
    match from_mp4_bytes_to_value(bytes, name_tag.clone()) {
        Ok(x) => match x {
            Value {
                value: UntaggedValue::Table(list),
                ..
            } => Ok(list.into_iter().map(ReturnSuccess::value).collect()),
            _ => Ok(vec![ReturnSuccess::value(x)]),
        },
        Err(_) => Err(ShellError::labeled_error(
            "Could not parse as MP4",
            "input cannot be parsed as MP4",
            &name_tag,
        )),
    }
}
