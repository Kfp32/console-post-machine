use std::collections::HashMap;
use std::thread;
use std::fs;
use ansi_term::Colour::{Red};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use tabled::{builder::Builder, settings::{Style, Color, Modify, object::Rows,}};
use std::time::Duration;
use std::process;
use std::io;

struct Tape {
  map: HashMap<i64, u8>,
  premap: HashMap<i64, u8>,
  cur: i64
}
struct Buffer {
  cmds: Vec<String>,
  cur: usize
} 
struct PostMachine {
  tape: Tape,
  buffer: Buffer,
  prev_error: Option<Error>,
  message: Option<String>,
  speedms: u64
}

enum Error {
  UndefinedUserCommand,
  InvalidUserLine,
  InvalidFormat(usize),
  UndefinedCommand(usize, String),
  InvalidLine(usize),
  CommandInEditMode,
  EmptyBufer,
  WrongSpeed
}

enum Command {
  LeftGoto(usize),
  RightGoto(usize), 
  MarkGoto(usize), 
  UnmarkGoto(usize),
  CheckOrGoto(usize, usize),
  End
}

impl PostMachine {
  pub fn new() -> Self {
    Self {
      tape: Tape {
        map: HashMap::new(),
        premap: HashMap::new(),
        cur: 0
      },
      buffer: Buffer {
        cmds: Vec::new(),
        cur: 0
      },
      prev_error: None,
      message: Some("/help - справка".to_string()),
      speedms: 300
    }
  }

  pub fn init(&mut self) {
    println!("Машина Поста v0.1");
    println!("Нажмите, чтобы начать...");
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).unwrap();

    self.coding();
  }

  fn printui(&mut self) {
    print!("\x1B[2J\x1B[1;1H");
    let mut builder = Builder::default();
    builder.push_record((self.tape.cur-8..=self.tape.cur+8).map(|i| i.to_string()));
    let mut v: Vec<String> = Vec::new();
    println!();
    for pos in self.tape.cur-8..=self.tape.cur+8 {
      let val = self.tape.map.get(&pos).unwrap_or(&0);
      if pos == self.tape.cur {
        let green_open  = Red.paint(">").to_string();
        let green_close = Red.paint("<").to_string();
        v.push(format!("{green_open}{val}{green_close}"));
      }
      else {
        v.push(val.to_string());
      }
    }
    builder.push_record(v);
    let mut table = builder.build();
    table.to_string();
    table.with(Style::modern());
    table.with(Modify::new(Rows::first()).with(Color::FG_GREEN));
    println!("{table}");
    
    match &self.message {
      None => {}
      Some(s) => {
        println!("{}", s);
      }
    }

    println!();
    for i in 0..self.buffer.cmds.len() {
      if i == self.buffer.cur.try_into().unwrap() {
        println!("> {} | {}", i+1, self.buffer.cmds[i]);
      }
      else {
        println!("  {} | {}", i+1, self.buffer.cmds[i]);
      }
    }
    println!();

    // if cfg!(debug_assertions) {
    //     println!("{:?} {}", self.buffer.cmds, k);
    // }

    match &self.prev_error {
      None => {}
      Some(err) => match err {
        Error::UndefinedUserCommand => {
          println!("Несуществующая команда!");
        }
        Error::InvalidUserLine => {
          println!("Несуществующая строка!");
        }
        Error::InvalidFormat(e) => {
          println!("{}: Неверный формат команды!", e);
        }
        Error::InvalidLine(e) => {
          println!("{}: Переход на несуществующую строку!", e);
        }
        Error::UndefinedCommand(e, s) => {
          println!("{}: Команда '{}' не найдена!", e, s);
        }
        Error::CommandInEditMode => {
          println!("Команды в режиме редактирования недостпупны!");
        }
        Error::WrongSpeed => {
          println!("Неправильный формат скорости!");
        }
        Error::EmptyBufer => {
          println!("Буфер команд пуст!");
        }
      }
    }
  }

  fn printhelp(&mut self) {
    print!("\x1B[2J\x1B[1;1H");
    let text = fs::read_to_string("src/help.txt").unwrap();
    println!("{text}");
    let mut st = String::new();
    io::stdin()
      .read_line(&mut st)
      .expect("Ошибка ввода");
  }

  fn coding(&mut self) {
    loop {
      let k = self.buffer.cmds.len();
      
      self.printui();

      let mut st = String::new();
      io::stdin()
        .read_line(&mut st)
        .expect("");

      st = st.trim().to_string();

      if st.is_empty() {
        self.message = None;
        continue;
      }

      if st.chars()
            .next().unwrap() == '/' {
            
            let cmd_usr = &st[1..];

            match cmd_usr.parse::<usize>() {
              Ok(v) => {
                if v <= k && v > 0 {
                  self.prev_error = None;
                    match self.coding_edit(v) {
                        Ok(_) => {}
                        Err(e) => {
                          self.prev_error = Some(e);
                          continue;
                        }
                    }
                }
                else {
                  self.prev_error = Some(Error::InvalidUserLine);
                  continue;
                }
              }
              Err(_) => {
                // TODO: доделай все обработки команд формата /...
                  match cmd_usr {
                    "r" => {
                      self.tape.map.clear();
                      self.tape.premap.clear();
                      self.message = Some("Лента очищена!".to_string());
                      self.prev_error = None;
                      continue;
                    }
                    "rr" => {
                      self.tape.map.clear();
                      self.tape.premap.clear();
                      self.buffer.cmds.clear();
                      self.message = Some("Лента и буфер команд очищены!".to_string());
                      self.prev_error = None;
                      continue;
                    }
                    "s" => {
                      if self.buffer.cmds.is_empty() {
                        self.prev_error = Some(Error::EmptyBufer);
                        self.message = None;
                        continue;
                      }
                      match self.compile_code() {
                        Ok(cmds) => {
                          self.prev_error = None;
                          self.message = None;
                          self.execute(&cmds);
                          self.tape.map = self.tape.premap.clone();
                        }
                        Err(e) => {
                          self.prev_error = Some(e);
                          self.message = None;
                          continue;
                        }
                      }                 
                    }
                    "sp" => {
                      self.message = None;
                      self.prev_error = None;
                      match self.new_sp() {
                          Ok(_) => {continue;}
                          Err(e) => {
                            self.prev_error = Some(e);
                            continue;
                          }

                      }
                    }
                    "help" => {
                      self.printhelp();
                      self.message = None;
                      self.prev_error = None;
                      continue;
                    }
                    "q" => {
                      process::exit(0);
                    }
                    "d" => {
                      self.buffer.cmds.pop();
                    }
                    _ => {
                      self.prev_error = Some(Error::UndefinedUserCommand);
                      continue;
                    }

                  }
                  
              }
            }
      }
      else if let Ok(n) = st.parse::<i64>() {
        if let Some(_) = self.tape.map.get(&n) {
          self.tape.map.remove(&n);
          self.tape.premap.remove(&n);
        } 
        else {
          self.tape.map.insert(n, 1);
          self.tape.premap.insert(n, 1);
        }
      }
      else {
        self.insert_command(&st);
      }
      self.prev_error = None;
      self.message = None;
    }

  }

  fn new_sp(&mut self) -> Result<(), Error> {
    self.printui();
    println!("Текущая скорость: {}мс", self.speedms);
    println!("Введите новую скорость исполнения (мс или ENTER для отмены):");
    let mut st = String::new();
    io::stdin()
      .read_line(&mut st)
      .expect("Ошибка ввода");

    if st.trim().is_empty() {
      return Ok(());
    }

    match st.trim().parse::<u64>() {
      Ok(n) => {
        self.speedms = n;
        self.message = Some(format!("Новая скорость: {}мс", self.speedms));
      }
      Err(_) => {
        return Err(Error::WrongSpeed);
      }
    }

    Ok(())
    
    
  }

  fn coding_edit(&mut self, k:usize) -> Result<(), Error>{
    self.printui();
    println!("Нажмите ENTER чтобы отменить редактирование");
    print!("{} ", k);

    let mut st = String::new();
    io::stdin()
      .read_line(&mut st)
      .expect("Ошибка ввода");
    
    st = st.trim().to_string();

    if !st.is_empty() {
      if st.chars()
            .next().unwrap() == '/' {
          return Err(Error::CommandInEditMode);
      }
      else {
        self.edit_command(&st, k as usize);
      }  
    } 
    Ok(())
  }

  fn insert_command(&mut self, command: &str) {
    self.buffer.cmds.push(command.to_owned());
  }

  fn edit_command(&mut self, command: &str, n: usize) {
    self.buffer.cmds[n-1] = command.to_string();
  }

  fn compile_code(&mut self) -> Result<Vec<Command>, Error>{
    let mut compile_commands: Vec<Command> = Vec::new();
    for (i, cmd) in self.buffer.cmds.iter().enumerate() {

      let mut chars = cmd.chars();
      let first = chars.next().unwrap_or('\0');
      let rest = chars.as_str();

      if rest.chars().next().map(|c| !c.is_whitespace()).unwrap_or(false) {
        return Err(Error::UndefinedCommand(i+1, cmd.to_string()));
      }

      match first {
        'v' | 'V' => {
          let num = rest.trim().parse::<usize>()
                    .map_err(|_| Error::InvalidFormat(i+1))?;
            
          if num > self.buffer.cmds.len() {
            return Err(Error::InvalidLine(i+1));
          }

          compile_commands.push(Command::MarkGoto(num-1));
        }
        'x' | 'X' => {
          let num = rest.trim().parse::<usize>()
                    .map_err(|_| Error::InvalidFormat(i+1))?;

          if num > self.buffer.cmds.len() {
            return Err(Error::InvalidLine(i+1));
          }

          compile_commands.push(Command::UnmarkGoto(num-1));

        }
        '?' => {
          if rest.len() < 2 {
              return Err(Error::UndefinedCommand(i+1, cmd.to_string()));
          }

          let mut parts = rest.split_whitespace();
          let a = parts.next().ok_or(Error::InvalidFormat(i+1))?;
          let b = parts.next().ok_or(Error::InvalidFormat(i+1))?;
          if parts.next().is_some() {
              return Err(Error::InvalidFormat(i+1)); // лишние аргументы
          }
          let num1 = a.parse::<usize>().map_err(|_| Error::InvalidFormat(i+1))?;
          let num2 = b.parse::<usize>().map_err(|_| Error::InvalidFormat(i+1))?;

          if num1 > self.buffer.cmds.len() || num2 > self.buffer.cmds.len() {
            return Err(Error::InvalidLine(i+1));
          }

          compile_commands.push(Command::CheckOrGoto(num1-1, num2-1));

        }
        '<' => {
          let num = rest.trim().parse::<usize>()
                    .map_err(|_| Error::InvalidFormat(i+1))?;

          if num > self.buffer.cmds.len() {
            return Err(Error::InvalidLine(i+1));
          }

          compile_commands.push(Command::LeftGoto(num-1));

        }
        '>' => {
          let num = rest.trim().parse::<usize>()
                    .map_err(|_| Error::InvalidFormat(i+1))?;

          if num > self.buffer.cmds.len() {
            return Err(Error::InvalidLine(i+1));
          }

          compile_commands.push(Command::RightGoto(num-1));

        }
        '!' => {
          if cmd.len() != 1 {
            return Err(Error::InvalidFormat(i+1));
          }
          compile_commands.push(Command::End);
          
        }
        _ => {
          return Err(Error::UndefinedCommand(i+1, cmd.to_string()));
        }
      }
    }
    
    Ok(compile_commands)
  }
 
  fn sleep(&mut self) {
    thread::sleep(Duration::from_millis(self.speedms));
  }

  fn execute(&mut self, cmds: &Vec<Command>) {
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_clone = stop_flag.clone();
    thread::spawn(move || {
        let mut buf = String::new();
        let _ = io::stdin().read_line(&mut buf);
        stop_clone.store(true, Ordering::SeqCst);
    });
    loop {

      let mut st = String::new();
      if stop_flag.load(Ordering::SeqCst) {
          self.message = Some("Программа завершена нажатием ENTER!".to_string());
          self.printui();
          io::stdin()
            .read_line(&mut st)
            .expect("Ошибка ввода");
          self.message = None;
          self.buffer.cur = 0;
          self.tape.cur = 0;
          self.tape.map.clear();
          return;
      }

      let ip = self.buffer.cur;
      let tpi = self.tape.cur;
      self.printui();
      self.sleep();

      match cmds[ip] {
        Command::LeftGoto(j) => {
          self.tape.cur -= 1;
          self.printui();
          self.buffer.cur = j;
          self.sleep();
        }
        Command::RightGoto(j) => {
          self.tape.cur += 1;
          self.printui();
          self.buffer.cur = j;
          self.sleep();
        }
        Command::MarkGoto(j) => {
          if let Some(_) = self.tape.map.get(&tpi) {
            self.sleep();
            self.message = Some("Запись в помеченное поле! Программа завершена...".to_string());
            self.printui();
            io::stdin()
              .read_line(&mut st)
              .expect("Ошибка ввода");
            self.message = None;
            self.buffer.cur = 0;
            self.tape.cur = 0;
            self.tape.map.clear();
            return;
          }
          self.tape.map.insert(tpi, 1);
          self.printui();
          self.sleep();
          self.buffer.cur = j;

        }
        Command::UnmarkGoto(j) => {
          if self.tape.map.get(&tpi).is_none() {
            self.sleep();
            self.message = Some("Стирание несуществующей метки! Программа завершена...".to_string());
            self.printui();
            io::stdin()
              .read_line(&mut st)
              .expect("Ошибка ввода");
            self.message = None;
            self.buffer.cur = 0;
            self.tape.cur = 0;
            self.tape.map.clear();
            return;
          }
          self.tape.map.remove(&tpi);
          self.printui();
          self.sleep();
          self.buffer.cur = j;

        }
        Command::CheckOrGoto(j1, j2) => {
          if let Some(_) = self.tape.map.get(&tpi) {
            self.buffer.cur = j1;
          }
          else {
            self.buffer.cur = j2;
          }
        }
        Command::End => {
          self.sleep();
          self.message = Some("Замечена команда стоп! Завершаю программу...".to_string());
          self.printui();
          io::stdin()
            .read_line(&mut st)
            .expect("Ошибка ввода");
          self.message = None;
          self.buffer.cur = 0;
          self.tape.cur = 0;
          self.tape.map.clear();
          return;
        }
      }

      

    }
  }
}


fn main() {
  let mut machine = PostMachine::new();
  machine.init();
}


