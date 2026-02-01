use std::collections::HashMap;
use std::process;
use std::io;

struct Tape {
  map: HashMap<i64, u8>,
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
  message: Option<String>
}

enum Error {
  UndefinedUserCommand,
  InvalidUserLine,
  InvalidFormat(usize),
  UndefinedCommand(usize, String),
  InvalidLine(usize),
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
        cur: 0
      },
      buffer: Buffer {
        cmds: Vec::new(),
        cur: 0
      },
      prev_error: None,
      message: Some("/h - список команд".to_string())
    }
  }

  pub fn init(&mut self) {
    println!("Машина Поста v0.1");
    println!("Нажмите, чтобы начать...");
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).unwrap();

    self.coding();
  }

  fn coding(&mut self) {
    loop {
      let mut k = self.buffer.cur;
      print!("\x1B[2J\x1B[1;1H");
      self.print_state();
      
      match &self.message {
        None => {}
        Some(s) => {
          println!("{}", s);
        }
      }

      println!();
      self.print_buffer();
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
        }
      }

      let mut st = String::new();
      io::stdin()
        .read_line(&mut st)
        .expect("Ошибка ввода");

      st = st.trim().to_string();

      if st.is_empty() {
        continue;
      }


      if st.chars()
            .next() == Some('/') {
            
            let cmd_usr = &st[1..];

            match cmd_usr.parse::<usize>() {
              Ok(v) => {
                if v <= k && v > 0 {
                    self.coding_edit(v);
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

                    }
                    "s" => {
                      match self.compile_code() {
                        Ok(_) => {

                        }
                        Err(e) => {
                          self.prev_error = Some(e);
                          continue;
                        }
                      }                 

                    }
                    "e" => {

                    }
                    "h" => {
                      self.message = Some("Дима лох".to_string());
                      self.prev_error = None;
                      continue;
                    }
                    "q" => {
                      process::exit(0);
                    }
                    "d" => {
                      self.buffer.cmds.pop();
                      self.buffer.cur -= 1;
                    }
                    _ => {
                      self.prev_error = Some(Error::UndefinedUserCommand);
                      continue;
                    }

                  }
                  
              }
            }
      }
      else {
        self.insert_command(&st);
      }
      self.prev_error = None;
      self.message = None;
    }

  }

  fn coding_edit(&mut self, k:usize) {
    print!("\x1B[2J\x1B[1;1H");
    self.print_state();
    println!();
    self.print_buffer();
    println!();
    println!("Нажмите ENTER чтобы отменить редактирование");
    print!("{} ", k);

    let mut st = String::new();
    io::stdin()
      .read_line(&mut st)
      .expect("Ошибка ввода");
    
    st = st.trim().to_string();

    if !st.is_empty() {
      self.edit_command(&st, k as usize);
    } 
  }

  fn insert_command(&mut self, command: &str) {
    self.buffer.cmds.push(command.to_owned());
    self.buffer.cur += 1;
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

  fn print_state(&self) {
    for pos in self.tape.cur-8..=self.tape.cur+8 {
      let val = self.tape.map.get(&pos).unwrap_or(&0);
      print!("{}", val);
    }
    println!();
    println!("        ^")
  }

  fn print_buffer(&self) {
    for i in 0..self.buffer.cmds.len() {
      if (i == self.tape.cur.try_into().unwrap()) {
        println!("> {} | {}", i+1, self.buffer.cmds[i]);
      }
      else {
        println!("  {} | {}", i+1, self.buffer.cmds[i]);
      }
    }
  }

}


fn main() {
  let mut machine = PostMachine::new();
  machine.init();
}


