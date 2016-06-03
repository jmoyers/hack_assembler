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
  iter: Peekable<Chars<'a>>,
  curr: char,
  state: State,

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
      iter: iter,
      curr: curr,
      state: StartLine,
      
      dest: String::new(),
      jump: String::new(),
      comp: String::new(),
      addr: String::new(),

      command_type: Invalid
    }
  }

  fn eof(&mut self) -> bool {
    self.iter.peek().is_none()
  }

  fn newline(&self) -> bool {
    self.curr == '\n' || self.curr == '\r'
  }

  fn should_ignore(c: &char) -> bool {
    match *c {
      ' ' | '\t' | '\r'  => true,
      _                  => false
    }
  }

  fn get_addr(&self) -> String {
    let mut command : String  = String::with_capacity(16);
    let length = self.addr.len() - 1;
    let mut total : u16 = 0;

    for (i, c) in self.addr.chars().enumerate() {
      total += (c as u16 - 48) * 10u16.pow(length as u32 - i as u32);
    }

    while total > 0 {
      let remainder = total % 2;
      command.push((remainder as u8 + 48) as char);
      total = total / 2;
    }

    // left pad
    while command.len() < 16 {
      command = command + "0";
    }

    command.chars().rev().collect()
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

    loop {
      let c = self.curr;

      if Parser::should_ignore(&c) {
        self.bump();
        continue;
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
          self.bump();
          self.addr.push(c);
        },
        InCCompare => {
          self.bump();

          self.state = match c {
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
          self.bump();
          self.jump.push(c);
        },
        _ => self.bump()
      };

      // Check for newlines, skip them if they're consecutive
      // Also handles the windows case of \r\n
      if self.newline() || self.eof() { 
        self.state = StartLine;
        while !self.eof() && self.newline() { 
          self.bump() 
        }
        break; 
      }
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
  let mut out : Vec<String> = Vec::new();

  while !p.eof() {
    p.advance();

    if p.command_type == A {
      out.push(p.get_addr());
    }

    if p.command_type == C {
      let comp = l.comp(&p.comp).unwrap();
      let dest = l.dest(&p.dest).unwrap();
      let jump = l.jump(&p.jump).unwrap();
      out.push("111".to_string() + comp + dest + jump);
    }
  }

  for line in out {
    println!("{}", line);
  }
}
