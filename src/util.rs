pub(crate) fn u32_to_string(value: u32) -> String {
    let bytes = value.to_be_bytes();
    // bytes to string
    let mut string = String::new();
    for byte in bytes.iter() {
        string.push(char::from(*byte));
    }
    string
}

pub(crate) fn sniff_encoded_image_dimensions(data: &[u8]) -> Option<(&'static str, u32, u32)> {
    if data.len() >= 24 && data.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) {
        let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
        let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
        return Some(("image/png", width, height));
    }

    if data.len() >= 4 && data[0] == 0xff && data[1] == 0xd8 {
        let mut offset = 2usize;
        while offset + 9 < data.len() {
            if data[offset] != 0xff {
                offset += 1;
                continue;
            }
            let marker = data[offset + 1];
            offset += 2;
            if marker == 0xd8 || marker == 0xd9 {
                continue;
            }
            if offset + 1 >= data.len() {
                break;
            }
            let segment_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
            if segment_len < 2 || offset + segment_len > data.len() {
                break;
            }
            if matches!(
                marker,
                0xc0 | 0xc1
                    | 0xc2
                    | 0xc3
                    | 0xc5
                    | 0xc6
                    | 0xc7
                    | 0xc9
                    | 0xca
                    | 0xcb
                    | 0xcd
                    | 0xce
                    | 0xcf
            ) {
                if offset + 7 >= data.len() {
                    break;
                }
                let height = u16::from_be_bytes([data[offset + 3], data[offset + 4]]) as u32;
                let width = u16::from_be_bytes([data[offset + 5], data[offset + 6]]) as u32;
                return Some(("image/jpeg", width, height));
            }
            offset += segment_len;
        }
    }

    None
}

/*
uint32
CalcTableChecksum(uint32 *Table, uint32 Length)
{
uint32 Sum = 0L;
uint32 *Endptr = Table+((Length+3) & ~3) / sizeof(uint32);
while (Table < EndPtr)
    Sum += *Table++;
return Sum;
} */

#[allow(dead_code)]
pub(crate) fn check_sum(table: Vec<u8>) -> u32 {
    let mut sum = 0;
    for i in 0..table.len() / 4 {
        let offset = i * 4;
        let mut bytes = [0; 4];
        for j in 0..4 {
            bytes[j] = table[offset + j];
        }
        let number: u32 = u32::from_be_bytes(bytes);
        sum += number;
    }
    let remain = table.len() % 4;
    if remain > 0 {
        let mut bytes = [0; 4];
        for i in 0..remain {
            bytes[i] = table[table.len() - remain + i];
        }
        let number: u32 = u32::from_be_bytes(bytes);
        sum += number;
    }
    sum
}
