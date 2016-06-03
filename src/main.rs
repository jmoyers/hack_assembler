use std::env;
use std::process::exit;

use std::io::Read;
use std::fs::File;
use std::collections::HashMap;
use std::str::Chars;
use std::iter::Peekable;
use std::str;

use self::State::*;
use self::CommandType::*;


struct SymbolTable {
  lookup: HashMap<String, String>
}

impl SymbolTable {
  fn new() -> SymbolTable {
    let mut l = HashMap::new();

    l.insert(String::from("R0"), SymbolTable::u16_to_bin(0));
    l.insert(String::from("R1"), SymbolTable::u16_to_bin(1));
    l.insert(String::from("R2"), SymbolTable::u16_to_bin(2));
    l.insert(String::from("R3"), SymbolTable::u16_to_bin(3));
    l.insert(String::from("R4"), SymbolTable::u16_to_bin(4));

    SymbolTable {
      lookup: l
    }
  }

  fn is_valid_address(addr : &String) -> bool {
    addr.chars().fold(true, |is_number, c| {
      is_number && (c as u8) >= 48 && (c as u8) <= 57
    })
  }

  fn string_to_u16(num : & String) -> u16 {
    let length = num.len() - 1;
    let mut total : u16 = 0;

    for (i, c) in num.chars().enumerate() {
      total += (c as u16 - 48) * 10u16.pow(length as u32 - i as u32);
    }

    total
  }

  fn u16_to_bin(input : u16) -> String {
    let mut command : String  = String::with_capacity(16);
    let mut num = input;

    while num > 0 {
      let remainder = num % 2;
      command.push((remainder as u8 + 48) as char);
      num = num / 2;
    }

    // left pad
    while command.len() < 16 {
      command = command + "0";
    }

    command.chars().rev().collect()
  }

  fn insert(&mut self, key : &String, val : &String) {
    self.lookup.insert(key.clone(), val.clone());
  }

  fn get(&self, key : &String) -> Option<&String>{
    self.lookup.get(key)
  }
}

struct CommandLookup<'a> {
  destination: HashMap<&'a str, &'a str>,
  compare: HashMap<&'a str, &'a str>,
  jump: HashMap<&'a str, &'a str>
}

impl<'a> CommandLookup<'a> {
  fn new() -> CommandLookup<'a> {
    let mut c = HashMap::new();

    // Machine language that uses ascii 😪
    c.insert("0",  "0101010");
    c.insert("1",  "0111111");
    c.insert("-1", "0111010");
    c.insert("D",  "0001100");
    c.insert("A",  "0110000");
    c.insert("M",  "1110000");
    c.insert("!D", "0001101");
    c.insert("!A", "0110001");
    c.insert("!M", "1110001");
    c.insert("-D", "0001111");
    c.insert("-A", "0110011");
    c.insert("-M", "1110011");
    c.insert("D+1","0011111");
    c.insert("A+1","0110111");
    c.insert("M+1","1110111");
    c.insert("D-1","0001110");
    c.insert("A-1","0110010");
    c.insert("M-1","1110010");
    c.insert("D+A","0000010");
    c.insert("D+M","1000010");
    c.insert("D-A","0010011");
    c.insert("D-M","1010011");
    c.insert("A-D","0000111");
    c.insert("M-D","1000111");
    c.insert("D&A","0000000");
    c.insert("D&M","1000000");
    c.insert("D|A","0010101");
    c.insert("D|M","1010101");

    let mut d = HashMap::new();

    d.insert("",    "000");
    d.insert("M",   "001");
    d.insert("D",   "010");
    d.insert("MD",  "011");
    d.insert("A",   "100");
    d.insert("AM",  "101");
    d.insert("AD",  "110");
    d.insert("AMD", "111");

    let mut j = HashMap::new();

    j.insert("",    "000");
    j.insert("JGT", "001");
    j.insert("JEQ", "010");
    j.insert("JGE", "011");
    j.insert("JLT", "100");
    j.insert("JNE", "101");
    j.insert("JLE", "110");
    j.insert("JMP", "111");
    
    CommandLookup {
      compare: c,
      destination: d,
      jump: j
    }
  }

  fn dest(&self, d : &String) -> Option<&&str> {
    self.destination.get(d.as_str())
  }

  fn comp(&self, c : &String) -> Option<&&str> {
    self.compare.get(c.as_str())
  }

  fn jump(&self, j : &String) -> Option<&&str> {
    self.jump.get(j.as_str())
  }
}

enum State {
  StartLine,
  InComment,
  InLabel,

  InACommand,

  InCCompare,
  InCJump
}

#[derive(PartialEq)]
enum CommandType {
  A,
  C,
  Label,
  Comment,
  Invalid
}

struct Parser<'a> {
  input: &'a String,
  iter: Peekable<Chars<'a>>,
  curr: char,
  state: State,
  count: u16,

  dest: String,
  jump: String,
  comp: String,
  addr: String,

  command_type: CommandType
}

impl<'a> Parser<'a> {
  fn new(input: &'a String) -> Parser<'a> {
    let mut iter = input.chars().peekable();
    let curr = iter.next().unwrap();

    Parser {
      input: input,
      iter: iter,
      curr: curr,
      state: StartLine,
      count: 0,
      
      dest: String::new(),
      jump: String::new(),
      comp: String::new(),
      addr: String::new(),

      command_type: Invalid
    }
  }

  fn reset(&mut self) {
    self.iter = self.input.chars().peekable();
    self.curr = self.iter.next().unwrap();
    self.count = 0;
    self.state = StartLine;
    self.dest.clear();
    self.jump.clear();
    self.comp.clear();
    self.addr.clear();
    self.command_type = Invalid;
  }

  fn eof(&mut self) -> bool {
    self.iter.peek().is_none()
  }

  fn newline(&self) -> bool {
    self.curr == '\n' || self.curr == '\r'
  }

  fn should_ignore(c: &char) -> bool {
    match *c {
      ' ' | '\t' => true,
      _          => false
    }
  }


  fn bump(&mut self) {
    if self.eof() { return; }
    self.curr = self.iter.next().unwrap();
  }

  fn advance(&mut self) {
    self.addr.clear();
    self.comp.clear();
    self.jump.clear();
    self.dest.clear();
    self.command_type = Invalid;

    loop {
      let c = self.curr;

      if Parser::should_ignore(&c) {
        self.bump();
        continue;
      }

      // Check for newlines, skip them if they're consecutive
      // Also handles the windows case of \r\n
      if self.newline() || self.eof() { 
        self.state = StartLine;
        while !self.eof() && self.newline() { 
          self.bump() 
        }
        if self.command_type == C || self.command_type == A {
          self.count += 1;
        }
        break; 
      }

      match self.state {
        StartLine => {
          self.state = match c {
            '@' => { 
              self.bump(); 
              self.command_type = A;
              InACommand 
            },
            '(' => { 
              self.bump(); 
              self.command_type = Label;
              InLabel
            },
            '/' | '*' => { 
              self.bump(); 
              self.command_type = Comment;
              InComment 
            },
            _  => {
              self.command_type = C;
              InCCompare
            }
          };
        },
        InACommand => {
          self.state = match c {
            '/' => InComment,
            _ => {
              self.bump();
              self.addr.push(c);
              InACommand
            }
          }
        },
        InCCompare => {
          self.bump();

          self.state = match c {
            '/' => InComment,
            '=' => {
              // '=' terminates destination chunk 
              // we've been defaulting to compare, tho
              self.dest.clone_from(&self.comp);
              self.comp.clear();
              InCCompare
            },
            ';' => {
              // ';' terminates compare chunk
              InCJump
            },
            _ => { 
              // compare is the default case as in A;JEQ
              self.comp.push(c);
              InCCompare
            }
          }
        },
        InCJump => {
          self.state = match c {
            '/' => InComment,
            _ => {
              self.bump();
              self.jump.push(c);
              InCJump
            }
          }
        },
        InLabel => {
          self.state = match c {
            '/' => InComment, 
            ')' => { 
              self.bump(); InLabel
            },
            _ => {
              self.bump();
              self.addr.push(c);
              InLabel
            }
          }
        },
        _ => self.bump()
      };
    }
  }
}

fn main() {
  let fname = match env::args().nth(1) {
    None => {
      println!("Usage: hack_assembler [file]");
      exit(0);
    },
    Some(fname) => fname,
  };

  let mut file = match File::open(fname) {
    Ok(f) => f,
    Err(_) => {
      println!("Error: Cannot open file.");
      exit(0);
    }
  };
  
  let mut contents = String::new();

  match file.read_to_string(&mut contents) {
    Err(_) => {
      println!("Error: File read interrupted.");
      exit(0);
    },
    _ => {}
  }

  let mut p = Parser::new(&contents);
  let l = CommandLookup::new();
  let mut s = SymbolTable::new();
  let mut out : Vec<String> = Vec::new();

  // Symbol pass
  while !p.eof() {
    p.advance();

    if p.command_type == Label {
      s.insert(&p.addr, &SymbolTable::u16_to_bin(p.count));
    }
  }

  p.reset();

  // Assemble pass
  while !p.eof() {
    p.advance();

    if p.command_type == A {
      if SymbolTable::is_valid_address(&p.addr) {
        let a_cmd = SymbolTable::u16_to_bin(SymbolTable::string_to_u16(&p.addr));
        out.push(a_cmd);
      } else {
        match s.get(&p.addr) {
          Some(addr) => {
            out.push(addr.clone());
          },
          None => {
            println!("Unknown symbol {}:'{}'", p.count, p.addr);
            exit(0);
          }
        }
      }
    }

    if p.command_type == C {
      let comp = match l.comp(&p.comp) {
        Some(f) => f,
        None => {
          println!("Source '{}' is no good", p.comp);
          exit(0);
        }
      };
      let dest = match l.dest(&p.dest) {
        Some(f) => f,
        None => {
          println!("Source '{}' is no good", p.dest);
          exit(0);
        }
      };
      let jump = match l.jump(&p.jump) {
        Some(f) => f,
        None => {
          println!("Source '{}' is no good", p.jump);
          exit(0);
        }
      };
      out.push("111".to_string() + comp + dest + jump);
    }
  }

  for line in out {
    println!("{}", line);
  }
}
