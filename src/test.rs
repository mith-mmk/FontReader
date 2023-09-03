mod tests {

  #[cfg(target_feature = "impl")]
  fn convert() {
    use crate::fontheader::fixed_to_f32;
    use crate::fontheader::f2dot14_to_f32;
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

  #[test]
  fn operand_encoding_test() -> Result<(), Box<dyn std::error::Error>>{
     use crate::opentype::outline::cff::operand_encoding;
      let b = [0x8b];
      let value = operand_encoding(&b);
      assert_eq!(value, Some(0_f64));
      let b = [0xef];
      let value = operand_encoding(&b);
      assert_eq!(value, Some(100_f64));
      let b = [0x27];
      let value = operand_encoding(&b);
      assert_eq!(value, Some(-100_f64));
      let b = [0xfa, 0x7c];
      let value = operand_encoding(&b);
      assert_eq!(value, Some(1000_f64));
      let b = [0xfe, 0x7c];
      let value = operand_encoding(&b);
      assert_eq!(value, Some(-1000_f64));
      let b = [0x1c, 0x27, 0x10];
      let value = operand_encoding(&b);
      assert_eq!(value, Some(10000_f64));
      let b = [0x1c, 0xd8, 0xf0];
      let value = operand_encoding(&b);
      assert_eq!(value, Some(-10000_f64));
      let b = [0x1d, 0x00, 0x01, 0x86, 0xa0];
      let value = operand_encoding(&b);
      assert_eq!(value, Some(100000_f64));
      let b = [0x1d, 0xff, 0xfe, 0x79, 0x60];
      let value = operand_encoding(&b);
      assert_eq!(value, Some(-100000_f64));
      let b = [31];
      let value = operand_encoding(&b);
      assert_eq!(value, None);
      let b = [0x1e, 0x2e, 0xa2, 0x5f];
      let value = operand_encoding(&b);
      // 2 - .25
      assert_eq!(value, None);
      let b = [0x1e, 0xe2, 0xa2, 0x5f];
      let value = operand_encoding(&b);
      assert_eq!(value, Some(-2.25_f64));
      // -0.140541e-3
      let b = [0x1e, 0x0a, 0x14, 0x05, 0x41, 0xc3, 0xff];
      let value = operand_encoding(&b);
      assert_eq!(value, Some(-0.0140541_f64));
      

      Ok(())
  }

}