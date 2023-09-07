pub(crate) fn u32_to_string(value: u32) -> String {
    let bytes = value.to_be_bytes();
    // bytes to string
    let mut string = String::new();
    for byte in bytes.iter() {
        string.push(char::from(*byte));
    }
    string
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
