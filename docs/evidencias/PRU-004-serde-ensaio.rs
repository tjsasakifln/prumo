//! Contraprova de persistência f64, usando serde_json 1.0.154.
//! Executar sem e com a feature serde_json/float_roundtrip.
//! Manifesto e resultados estão em docs/componentes/PRU-004.md.

fn main() {
    let mut state = 0x123456789abcdef0u64;
    let mut count = 0;
    for _ in 0..100000 {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let value = f64::from_bits((1023u64 << 52) | (state & ((1u64 << 52) - 1)));
        let encoded = serde_json::to_string(&value).unwrap();
        let restored: f64 = serde_json::from_str(&encoded).unwrap();
        let independently_parsed: f64 = encoded.parse().unwrap();
        assert_eq!(value.to_bits(), independently_parsed.to_bits());
        if value.to_bits() != restored.to_bits() {
            if count < 5 {
                println!(
                    "{encoded}: {:#x} -> {:#x}; output {}",
                    value.to_bits(),
                    restored.to_bits(),
                    serde_json::to_string(&restored).unwrap(),
                );
            }
            count += 1;
        }
    }
    println!("mismatches={count}/100000");
}
