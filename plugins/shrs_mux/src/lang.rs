use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::format,
    io::{BufRead, BufReader, Read, Write},
    ops::Add,
    os::unix::process::ExitStatusExt,
    process::{Child, ChildStderr, ChildStdout, Command, ExitStatus, Stdio},
};

use shrs::prelude::*;

use crate::{
    interpreter::{read_err, read_out},
    MuxState,
};

pub struct MuxLang {
    langs: HashMap<String, Box<dyn Lang>>,
}

impl MuxLang {
    pub fn new(langs: HashMap<String, Box<dyn Lang>>) -> Self {
        // TODO should be configurable later
        Self { langs }
    }
}

impl Lang for MuxLang {
    fn eval(
        &self,
        sh: &Shell,
        ctx: &mut Context,
        rt: &mut Runtime,
        cmd: String,
    ) -> anyhow::Result<CmdOutput> {
        let lang_name = match ctx.state.get::<MuxState>() {
            Some(state) => &state.lang,
            None => return Ok(CmdOutput::error()),
        };
        // TODO maybe return error if we can't find a lang

        if let Some(lang) = self.langs.get(lang_name) {
            return lang.eval(sh, ctx, rt, cmd);
        }

        Ok(CmdOutput::error())
    }

    fn name(&self) -> String {
        "mux".to_string()
    }

    fn needs_line_check(&self, cmd: String) -> bool {
        false
    }
}

pub struct NuLang {}

impl NuLang {
    pub fn new() -> Self {
        Self {}
    }
}

impl Lang for NuLang {
    fn eval(
        &self,
        sh: &Shell,
        ctx: &mut Context,
        rt: &mut Runtime,
        cmd: String,
    ) -> shrs::anyhow::Result<CmdOutput> {
        let mut handle = Command::new("nu")
            .args(vec!["-c", &cmd])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let output = handle.wait_with_output()?;
        // ctx.out.print(output.stdout);

        Ok(CmdOutput::success())
    }

    fn name(&self) -> String {
        "nu".to_string()
    }

    fn needs_line_check(&self, cmd: String) -> bool {
        false
    }
}

pub struct PythonLang {}

impl PythonLang {
    pub fn new() -> Self {
        Self {}
    }
}

impl Lang for PythonLang {
    fn eval(
        &self,
        sh: &Shell,
        ctx: &mut Context,
        rt: &mut Runtime,
        cmd: String,
    ) -> shrs::anyhow::Result<CmdOutput> {
        let mut handle = Command::new("python3")
            .args(vec!["-c", &cmd])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let output = handle.wait_with_output()?;

        Ok(CmdOutput::success())
    }

    fn name(&self) -> String {
        "python".to_string()
    }

    fn needs_line_check(&self, cmd: String) -> bool {
        false
    }
}

pub struct BashLang {
    pub instance: RefCell<Child>,
}

impl BashLang {
    pub fn new() -> Self {
        Self {
            instance: RefCell::new(
                Command::new("bash")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .expect("Failed to start bash lol"),
            ),
        }
    }
}

impl Lang for BashLang {
    fn eval(
        &self,
        sh: &Shell,
        ctx: &mut Context,
        rt: &mut Runtime,
        cmd: String,
    ) -> shrs::anyhow::Result<CmdOutput> {
        let mut instance = self.instance.borrow_mut();
        let stdin = instance.stdin.as_mut().expect("Failed to open stdin");

        let cd_statement = format!("cd {}\n", rt.working_dir.to_string_lossy());

        stdin
            .write_all(cd_statement.as_bytes())
            .expect("unable to set var");

        for (k, v) in rt.env.iter() {
            let export_statement = format!("export {}={:?}\n", k, v);
            stdin
                .write_all(export_statement.as_bytes())
                .expect("unable to set var");
        }
        stdin
            .write_all((cmd + ";echo $?'\x1A'; echo '\x1A' >&2\n").as_bytes())
            .expect("Bash command failed");

        let stdout_reader =
            BufReader::new(instance.stdout.as_mut().expect("Failed to open stdout"));
        let status = read_out(ctx, stdout_reader)?;

        let stderr_reader =
            BufReader::new(instance.stderr.as_mut().expect("Failed to open stdout"));
        read_err(ctx, stderr_reader)?;

        Ok(CmdOutput::new(status))
    }

    fn name(&self) -> String {
        "bash".to_string()
    }

    fn needs_line_check(&self, cmd: String) -> bool {
        false
    }
}
