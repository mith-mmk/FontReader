#[cfg(test)]
mod tests {
  #[test]
  use fontheader::fixed_to_f32;
  use fontheader::f2dot14_to_f32;
  fn convert() {
    /*1.999939	0x7fff	1	16383/16384
      1.75	0x7000	1	12288/16384
      0.000061	0x0001	0	1/16384
      0.0	0x0000	0	0/16384
      -0.000061	0xffff	-1	16383/16384
      -2.0	0x8000	-2	0/16384 */
    let value = f2dot14_to_f32(0x7fff);
    assert_eq!(value, 1.999939);
    let value = f2dot14_to_f32(0x7000);
    assert_eq!(value, 1.75);
    let value = f2dot14_to_f32(0x0001);
    assert_eq!(value, 0.000061);
    let value = f2dot14_to_f32(0x0000);
    assert_eq!(value, 0.0);
    let value = f2dot14_to_f32(0xffff);
    assert_eq!(value, -0.000061);
    let value = f2dot14_to_f32(0x8000);
    assert_eq!(value, -2.0);
    let value = fixed_to_f32(0x7fff_ffff);
    assert_eq!(value, 1.9999998807907104);
    let value = fixed_to_f32(0x7000_0000);
    assert_eq!(value, 1.75);
    let value = fixed_to_f32(0x0000_0001);
    assert_eq!(value, 0.00000005960464477539);
    let value = fixed_to_f32(0x0000_0000);
    assert_eq!(value, 0.0);
    let value = fixed_to_f32(0xffff_ffff);
    assert_eq!(value, -0.00000005960464477539);
    let value = fixed_to_f32(0x8000_0000);
    assert_eq!(value, -2.0);
  }
}