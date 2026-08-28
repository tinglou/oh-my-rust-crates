//! hypervectorscan-demo - Comprehensive demonstration of hypervectorscan features
//!
//! This demo showcases:
//! - Block mode scanning (basic)
//! - Pattern with compile flags (CASELESS, DOTALL, etc.)
//! - Stream mode scanning
//! - Database serialization / deserialization
//! - Early termination
//! - Error handling

use std::sync::Arc;

use hypervectorscan::{
    BlockDatabase, BlockScanner, Flag, Pattern, Scan, StreamingDatabase, StreamingScanner,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║       hypervectorscan - Feature Demonstration            ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    demo_block_scanning()?;
    demo_pattern_flags()?;
    demo_stream_scanning()?;
    demo_serialization()?;
    demo_early_termination()?;
    demo_error_handling();

    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║                  Demo Complete! ✓                         ║");
    println!("╚═══════════════════════════════════════════════════════════╝");

    Ok(())
}

fn demo_block_scanning() -> Result<(), Box<dyn std::error::Error>> {
    println!("┌───────────────────────────────────────────────────────────┐");
    println!("│ 1. Block Mode Scanning                                   │");
    println!("└───────────────────────────────────────────────────────────┘");

    let patterns = vec![
        Pattern::new(b"hello".to_vec(), Flag::default(), None),
        Pattern::new(b"world".to_vec(), Flag::default(), None),
        Pattern::new(b"rust".to_vec(), Flag::default(), None),
    ];

    let db = Arc::new(BlockDatabase::new(patterns)?);
    let mut scanner = BlockScanner::new(db.clone())?;

    let data = b"hello world hello rust";
    println!("  Input:   {}", String::from_utf8_lossy(data));
    println!("  DB size: {} bytes", db.size()?);

    let mut matches = Vec::new();
    scanner.scan(data, |id, from, to, _flags| {
        matches.push((id, from, to));
        Scan::Continue
    })?;

    println!("  Matches: {} found", matches.len());
    for (id, from, to) in &matches {
        println!("    pattern[{id}] @ [{from}..{to})");
    }
    println!();
    Ok(())
}

fn demo_pattern_flags() -> Result<(), Box<dyn std::error::Error>> {
    println!("┌───────────────────────────────────────────────────────────┐");
    println!("│ 2. Pattern Compile Flags                                 │");
    println!("└───────────────────────────────────────────────────────────┘");

    // CASELESS: case-insensitive matching
    {
        let patterns = vec![Pattern::new(b"HELLO".to_vec(), Flag::CASELESS, Some(0))];
        let db = Arc::new(BlockDatabase::new(patterns)?);
        let mut scanner = BlockScanner::new(db)?;

        let data = b"HeLLo world";
        println!("  Input:   {}", String::from_utf8_lossy(data));
        println!("  Pattern: \"HELLO\" with CASELESS flag");

        let mut matches = Vec::new();
        scanner.scan(data, |id, from, to, _flags| {
            matches.push((id, from, to));
            Scan::Continue
        })?;
        println!("  Matched: {:?}", !matches.is_empty());
    }

    println!();

    // DOTALL: . matches newline
    {
        let patterns = vec![Pattern::new(b"a.b".to_vec(), Flag::DOTALL, Some(0))];
        let db = Arc::new(BlockDatabase::new(patterns)?);
        let mut scanner = BlockScanner::new(db)?;

        let data = b"a\nb";
        println!(
            "  Input:   \"{}\"",
            String::from_utf8_lossy(data).replace('\n', "\\n")
        );
        println!("  Pattern: \"a.b\" with DOTALL flag");

        let mut matches = Vec::new();
        scanner.scan(data, |id, from, to, _flags| {
            matches.push((id, from, to));
            Scan::Continue
        })?;
        println!("  Matched: {:?}", !matches.is_empty());
    }

    println!();

    // MULTILINE: ^/$ match start/end of line
    {
        let patterns = vec![Pattern::new(b"^hello".to_vec(), Flag::MULTILINE, Some(0))];
        let db = Arc::new(BlockDatabase::new(patterns)?);
        let mut scanner = BlockScanner::new(db)?;

        let data = b"world\nhello rust";
        println!(
            "  Input:   \"{}\"",
            String::from_utf8_lossy(data).replace('\n', "\\n")
        );
        println!("  Pattern: \"^hello\" with MULTILINE flag");

        let mut matches = Vec::new();
        scanner.scan(data, |id, from, to, _flags| {
            matches.push((id, from, to));
            Scan::Continue
        })?;
        println!("  Match at: {}..{}", matches[0].1, matches[0].2);
    }

    println!();
    Ok(())
}

fn demo_stream_scanning() -> Result<(), Box<dyn std::error::Error>> {
    println!("┌───────────────────────────────────────────────────────────┐");
    println!("│ 3. Stream Mode Scanning                                  │");
    println!("└───────────────────────────────────────────────────────────┘");

    let patterns = vec![
        Pattern::new(b"hello".to_vec(), Flag::default(), None),
        Pattern::new(b"world".to_vec(), Flag::default(), None),
    ];

    let db = Arc::new(StreamingDatabase::new(patterns)?);
    let scanner = Arc::new(StreamingScanner::new(db.clone())?);
    let mut stream = scanner.open_stream()?;

    println!("  DB size:       {} bytes", db.size()?);
    println!("  Stream size:   {} bytes", db.stream_size()?);

    let chunks = [
        (b"foo bar hel" as &[u8], "foo bar hel"),
        (b"lo wor" as &[u8], "lo wor"),
        (b"ld baz" as &[u8], "ld baz"),
    ];

    for (i, (chunk, desc)) in chunks.iter().enumerate() {
        let mut matches = Vec::new();
        stream.scan(chunk, |id, from, to, _flags| {
            matches.push((id, from, to));
            Scan::Continue
        })?;
        println!("  Chunk {i} \"{desc}\": {} matches", matches.len());
    }

    // Close and check final matches
    let mut final_matches = Vec::new();
    stream.close(|id, from, to, _flags| {
        final_matches.push((id, from, to));
        Scan::Continue
    })?;
    println!("  Close:       {} final matches", final_matches.len());
    println!();
    Ok(())
}

fn demo_serialization() -> Result<(), Box<dyn std::error::Error>> {
    println!("┌───────────────────────────────────────────────────────────┐");
    println!("│ 4. Database Serialization / Deserialization              │");
    println!("└───────────────────────────────────────────────────────────┘");

    let patterns = vec![Pattern::new(
        b"hypervectorscan".to_vec(),
        Flag::default(),
        None,
    )];
    let db = BlockDatabase::new(patterns)?;
    let original_size = db.size()?;
    println!("  Original DB size: {} bytes", original_size);

    // Serialize
    let sdb = db.serialize()?;
    let serialized = sdb.to_vec();
    println!("  Serialized size:  {} bytes", serialized.len());
    println!(
        "  Deserialized required: {} bytes",
        sdb.deserialized_size()?
    );

    // Deserialize
    let db2 = BlockDatabase::deserialize(sdb)?;
    println!("  Deserialized DB size: {} bytes", db2.size()?);
    assert_eq!(original_size, db2.size()?);

    // Verify it still works
    let db2 = Arc::new(db2);
    let mut scanner = BlockScanner::new(db2)?;

    let data = b"hypervectorscan rules!";
    let mut matched = false;
    scanner.scan(data, |_id, _from, _to, _flags| {
        matched = true;
        Scan::Continue
    })?;
    println!("  Re-scan matched: {:?}", matched);
    println!();
    Ok(())
}

fn demo_early_termination() -> Result<(), Box<dyn std::error::Error>> {
    println!("┌───────────────────────────────────────────────────────────┐");
    println!("│ 5. Early Termination                                     │");
    println!("└───────────────────────────────────────────────────────────┘");

    let patterns = vec![Pattern::new(b"test".to_vec(), Flag::default(), None)];
    let db = Arc::new(BlockDatabase::new(patterns)?);
    let mut scanner = BlockScanner::new(db)?;

    let data = b"test test test test";

    // Without termination
    let mut count = 0;
    scanner.scan(data, |_id, _from, _to, _flags| {
        count += 1;
        Scan::Continue
    })?;
    println!("  Without termination: {} matches", count);

    // With early termination
    let mut count = 0;
    scanner.scan(data, |_id, _from, _to, _flags| {
        count += 1;
        if count >= 2 {
            Scan::Terminate
        } else {
            Scan::Continue
        }
    })?;
    println!("  With early termination: {} matches", count);
    println!();
    Ok(())
}

fn demo_error_handling() {
    println!("┌───────────────────────────────────────────────────────────┐");
    println!("│ 6. Error Handling                                        │");
    println!("└───────────────────────────────────────────────────────────┘");

    // Invalid regex
    match BlockDatabase::new(vec![Pattern::new(b"[".to_vec(), Flag::default(), None)]) {
        Ok(_) => println!("  Unexpected success"),
        Err(e) => println!("  Invalid regex: {e}"),
    }

    // Empty pattern
    match BlockDatabase::new(vec![Pattern::new(b"".to_vec(), Flag::default(), None)]) {
        Ok(_) => println!("  Unexpected success"),
        Err(e) => println!("  Empty pattern: {e}"),
    }

    println!();
}
