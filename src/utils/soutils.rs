pub fn get_so_name() -> &'static str {
  match std::env::consts::OS {
      "windows" => "win",
      _ => std::env::consts::OS
  }
}

pub fn get_arch() -> &'static str {
  match std::env::consts::ARCH {
      "x86_64" => "x64",
      "x86" => "x86",
      _ => std::env::consts::ARCH
  }
}