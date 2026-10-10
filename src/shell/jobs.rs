use crate::job::Job;
use std::process::Child;

use super::Shell;

impl Shell {
    pub fn add_job(&self, children: Vec<Child>, command_line: String) {
        if children.is_empty() {
            return;
        }
        let job = Job::new(self.jobs.borrow().len() + 1, children, command_line);
        if let Some(pid) = job.last_pid() {
            println!("[{}] {}", job.number, pid);
        }
        self.jobs.borrow_mut().push(job);
    }

    pub fn check_jobs(&self) {
        let mut jobs = self.jobs.borrow_mut();

        for job in jobs.iter_mut() {
            if job.running && job.is_done() {
                job.running = false;
            }
        }

        let len = jobs.iter().filter(|x| !x.running).count();

        for (idx, job) in jobs.iter().filter(|x| !x.running).enumerate() {
            let sign = if idx + 1 == len {
                '+'
            } else if idx + 2 == len {
                '-'
            } else {
                ' '
            };

            println!("[{}]{}  {:<24}{}", job.number, sign, "Done", job.command_line);
        }

        jobs.retain(|x| x.running);
    }
}
