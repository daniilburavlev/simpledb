use clap::Parser;
use common::DbResult;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::rc::Rc;
use table::SimpleDB;
use table::field_info::FieldInfo;
use table::scan::Scan;
use table::schema::Schema;

fn default_path() -> String {
    "data".to_string()
}

#[derive(Parser)]
struct Cli {
    #[clap(long, default_value = default_path())]
    path: PathBuf,
}

fn execute(db: &SimpleDB, query: &str) -> DbResult<()> {
    let q = query.to_lowercase();
    let tx = db.get_tx()?;
    if q.starts_with("select") {
        let result = db.query(&tx, query)?;
        let schema = result.schema()?;
        print_data(&schema, &result)?;
    } else {
        let result = db.execute(&tx, query)?;
        println!("executed: {}", result);
    }
    tx.commit()?;
    Ok(())
}

fn print_data(schema: &Schema, result: &Rc<dyn Scan>) -> DbResult<()> {
    let fields = schema.fields()?;
    let mut rows: Vec<Vec<String>> = vec![];
    while result.next()? {
        let mut row = Vec::with_capacity(fields.len());
        for (field, info) in &fields {
            match info {
                FieldInfo::Integer => row.push(result.get_i32(field)?.to_string()),
                FieldInfo::Varchar(_) => row.push(format!("'{}'", result.get_string(field)?)),
            }
        }
        rows.push(row);
    }
    let headers: Vec<String> = fields.into_iter().map(|(f, _)| f).collect();
    let mut widths: Vec<usize> = headers.iter().map(|f| f.len()).collect();
    for row in &rows {
        for i in 0..headers.len() {
            let cell_len = row.get(i).map(|s| s.len()).unwrap_or(0);
            if cell_len > widths[i] {
                widths[i] = cell_len;
            }
        }
    }
    for (i, h) in headers.iter().enumerate() {
        print!("{:width$} |", h, width = widths[i]);
    }
    println!();
    let total_width = widths.iter().sum::<usize>() + widths.len();
    println!("{}", "-".repeat(total_width));

    for row in rows {
        for i in 0..headers.len() {
            let cell = row.get(i).map(|s| s.as_str()).unwrap_or("");
            print!("{:width$} |", cell, width = widths[i]);
        }
        println!();
    }
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    let Ok(db) = SimpleDB::new(&cli.path) else {
        panic!("cannot create database");
    };
    let stdin = std::io::stdin();
    print_prefix();
    for line in stdin.lock().lines() {
        let Ok(line) = line else {
            panic!("error reading from stdin");
        };
        if let Err(e) = execute(&db, &line) {
            eprintln!("error: {}", e);
        }
        print_prefix();
    }
}

fn print_prefix() {
    print!("sql> ");
    std::io::stdout().flush().unwrap();
}
