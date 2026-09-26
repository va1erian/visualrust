//! Splits a `.ico` container into the `RT_ICON` images and one `RT_GROUP_ICON`
//! directory that Windows needs to show one icon for the app.
//!
//! The container format is `ICONDIR` + `ICONDIRENTRY[]` (see `WinUser.h`); each
//! entry's offset/length slices the payload, and the group directory replaces
//! the file offsets with the `nID`s the images are registered under.

use crate::error::PackError;
use crate::resources::{PatchOp, RT_GROUP_ICON, RT_ICON};

/// The group's resource id. Windows picks the group with the lowest id when the
/// shell asks for an executable's icon, so 1 is the safe, conventional choice.
const GROUP_ID: u16 = 1;
/// Resource ids for the images inside the group, starting at the same base.
const FIRST_IMAGE_ID: u16 = 1;

const ICONDIR_LEN: usize = 6;
const ICONDIRENTRY_LEN: usize = 16;
const GRPICONDIRENTRY_LEN: usize = 14;

/// One image extracted from the container.
struct IconImage {
    width: u8,
    height: u8,
    color_count: u8,
    planes: u16,
    bit_count: u16,
    bytes: Vec<u8>,
}

/// Turns `.ico` bytes into the resource writes that register it.
pub(crate) fn ops(ico: &[u8]) -> Result<Vec<PatchOp>, PackError> {
    let images = parse(ico)?;
    let mut ops = Vec::with_capacity(images.len() + 1);
    for (index, image) in images.iter().enumerate() {
        let id = FIRST_IMAGE_ID + u16::try_from(index).unwrap_or(0);
        ops.push(PatchOp {
            type_id: RT_ICON,
            name_id: id,
            language: 0,
            data: image.bytes.clone(),
        });
    }
    ops.push(PatchOp {
        type_id: RT_GROUP_ICON,
        name_id: GROUP_ID,
        language: 0,
        data: group_directory(&images)?,
    });
    Ok(ops)
}

fn parse(ico: &[u8]) -> Result<Vec<IconImage>, PackError> {
    if ico.len() < ICONDIR_LEN {
        return Err(invalid("too short for an ICONDIR"));
    }
    let reserved = word(ico, 0);
    let kind = word(ico, 2);
    let count = word(ico, 4) as usize;
    if reserved != 0 {
        return Err(invalid("ICONDIR reserved field is not zero"));
    }
    if kind != 1 {
        return Err(invalid("not an icon (type is not 1)"));
    }
    if count == 0 {
        return Err(invalid("contains no images"));
    }
    let table_end = ICONDIR_LEN + count * ICONDIRENTRY_LEN;
    if ico.len() < table_end {
        return Err(invalid("truncated ICONDIRENTRY table"));
    }

    let mut images = Vec::with_capacity(count);
    for index in 0..count {
        let entry = ICONDIR_LEN + index * ICONDIRENTRY_LEN;
        let length = dword(ico, entry + 8) as usize;
        let offset = dword(ico, entry + 12) as usize;
        let end = offset
            .checked_add(length)
            .ok_or_else(|| invalid("image range overflows"))?;
        if end > ico.len() {
            return Err(invalid("image extends past the end of the file"));
        }
        images.push(IconImage {
            width: ico[entry],
            height: ico[entry + 1],
            color_count: ico[entry + 2],
            planes: word(ico, entry + 4),
            bit_count: word(ico, entry + 6),
            bytes: ico[offset..end].to_vec(),
        });
    }
    Ok(images)
}

fn group_directory(images: &[IconImage]) -> Result<Vec<u8>, PackError> {
    let count = u16::try_from(images.len()).map_err(|_| invalid("too many images"))?;
    let mut out = Vec::with_capacity(ICONDIR_LEN + images.len() * GRPICONDIRENTRY_LEN);
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    for (index, image) in images.iter().enumerate() {
        let id = FIRST_IMAGE_ID + u16::try_from(index).unwrap_or(0);
        out.push(image.width);
        out.push(image.height);
        out.push(image.color_count);
        out.push(0); // reserved
        out.extend_from_slice(&image.planes.to_le_bytes());
        out.extend_from_slice(&image.bit_count.to_le_bytes());
        let length = u32::try_from(image.bytes.len()).map_err(|_| invalid("image is too large"))?;
        out.extend_from_slice(&length.to_le_bytes());
        out.extend_from_slice(&id.to_le_bytes());
    }
    Ok(out)
}

fn word(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn dword(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn invalid(reason: &str) -> PackError {
    PackError::InvalidIcon {
        reason: reason.to_owned(),
    }
}

/// A 1x1 32-bit icon: BITMAPINFOHEADER + one BGRA pixel + a padded AND mask.
/// Shared with the resource encoder's tests.
#[cfg(test)]
pub(crate) fn sample_ico() -> Vec<u8> {
    let mut dib = Vec::new();
    dib.extend_from_slice(&40u32.to_le_bytes()); // biSize
    dib.extend_from_slice(&1i32.to_le_bytes()); // biWidth
    dib.extend_from_slice(&2i32.to_le_bytes()); // biHeight: XOR + AND
    dib.extend_from_slice(&1u16.to_le_bytes()); // biPlanes
    dib.extend_from_slice(&32u16.to_le_bytes()); // biBitCount
    dib.extend_from_slice(&0u32.to_le_bytes()); // biCompression
    dib.extend_from_slice(&0u32.to_le_bytes()); // biSizeImage
    for _ in 0..4 {
        dib.extend_from_slice(&0i32.to_le_bytes());
    }
    dib.extend_from_slice(&[0, 0, 255, 255]); // one opaque red pixel (BGRA)
    dib.extend_from_slice(&[0, 0, 0, 0]); // AND mask, padded to 4 bytes

    let mut ico = Vec::new();
    ico.extend_from_slice(&0u16.to_le_bytes());
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.push(1); // width
    ico.push(1); // height
    ico.push(0); // color count
    ico.push(0); // reserved
    ico.extend_from_slice(&1u16.to_le_bytes()); // planes
    ico.extend_from_slice(&32u16.to_le_bytes()); // bit count
    ico.extend_from_slice(&(dib.len() as u32).to_le_bytes());
    ico.extend_from_slice(&(ICONDIR_LEN as u32 + ICONDIRENTRY_LEN as u32).to_le_bytes());
    ico.extend_from_slice(&dib);
    ico
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ops_carry_one_image_and_a_group() {
        let ico = sample_ico();
        let ops = ops(&ico).expect("valid ico");
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0].type_id, RT_ICON);
        assert_eq!(ops[0].name_id, 1);
        assert_eq!(ops[1].type_id, RT_GROUP_ICON);
        assert_eq!(ops[1].name_id, GROUP_ID);

        let group = &ops[1].data;
        assert_eq!(u16::from_le_bytes([group[4], group[5]]), 1, "one entry");
        let nid = u16::from_le_bytes([
            group[ICONDIR_LEN + GRPICONDIRENTRY_LEN - 2],
            group[ICONDIR_LEN + GRPICONDIRENTRY_LEN - 1],
        ]);
        assert_eq!(nid, 1, "entry points at RT_ICON id 1");
    }

    #[test]
    fn rejects_non_icons() {
        assert!(ops(b"nope").is_err());
        let mut png = sample_ico();
        png[2] = 2; // ICONDIR idType = cursor
        assert!(ops(&png).is_err());
    }
}
