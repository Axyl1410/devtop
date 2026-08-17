use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct ProcessHarvest {
    pub pid: u32,
    pub parent_pid: Option<u32>,
    pub name: String,
    pub cpu_usage: f32,
    pub memory_bytes: u64,
    pub virtual_memory_bytes: u64,
    pub memory_percent: f32,
    pub status: String,
    pub cmd: String,
    pub exe: String,
    pub cwd: String,
    pub user: String,
    pub run_time_secs: u64,
    pub ports: Vec<u16>,
    pub children: Vec<u32>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct ProcessTreeItem {
    pub process: ProcessHarvest,
    pub depth: usize,
    pub is_last_child: bool,
    pub prefix: String,
}

pub fn build_process_tree(
    processes: &[ProcessHarvest],
    search_query: &str,
) -> Vec<ProcessTreeItem> {
    let proc_map: HashMap<u32, ProcessHarvest> =
        processes.iter().map(|p| (p.pid, p.clone())).collect();

    let mut parent_to_children: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut roots: Vec<u32> = Vec::new();

    for (pid, proc) in &proc_map {
        match proc.parent_pid {
            Some(ppid) if proc_map.contains_key(&ppid) && ppid != *pid => {
                parent_to_children.entry(ppid).or_default().push(*pid);
            }
            _ => {
                roots.push(*pid);
            }
        }
    }

    // Sort roots & children by CPU usage descending
    roots.sort_by_key(|pid| {
        proc_map
            .get(pid)
            .map(|p| (-(p.cpu_usage * 100.0) as i64, p.pid))
            .unwrap_or((0, *pid))
    });
    for children in parent_to_children.values_mut() {
        children.sort_by_key(|pid| {
            proc_map
                .get(pid)
                .map(|p| (-(p.cpu_usage * 100.0) as i64, p.pid))
                .unwrap_or((0, *pid))
        });
    }

    let mut result = Vec::new();

    fn traverse(
        pid: u32,
        prefix: &str,
        is_last: bool,
        depth: usize,
        proc_map: &HashMap<u32, ProcessHarvest>,
        parent_to_children: &HashMap<u32, Vec<u32>>,
        result: &mut Vec<ProcessTreeItem>,
    ) {
        if let Some(proc) = proc_map.get(&pid) {
            let current_branch = if depth == 0 {
                String::new()
            } else if is_last {
                format!("{}└─ ", prefix)
            } else {
                format!("{}├─ ", prefix)
            };

            result.push(ProcessTreeItem {
                process: proc.clone(),
                depth,
                is_last_child: is_last,
                prefix: current_branch,
            });

            if let Some(children) = parent_to_children.get(&pid) {
                let next_prefix = if depth == 0 {
                    String::new()
                } else if is_last {
                    format!("{}   ", prefix)
                } else {
                    format!("{}│  ", prefix)
                };

                let child_count = children.len();
                for (i, child_pid) in children.iter().enumerate() {
                    let is_last_child = i == child_count - 1;
                    traverse(
                        *child_pid,
                        &next_prefix,
                        is_last_child,
                        depth + 1,
                        proc_map,
                        parent_to_children,
                        result,
                    );
                }
            }
        }
    }

    for root in roots {
        traverse(
            root,
            "",
            true,
            0,
            &proc_map,
            &parent_to_children,
            &mut result,
        );
    }

    if !search_query.is_empty() {
        let q = search_query.to_lowercase();
        result.retain(|item| {
            item.process.name.to_lowercase().contains(&q)
                || item.process.cmd.to_lowercase().contains(&q)
                || item.process.pid.to_string().contains(&q)
                || item.process.user.to_lowercase().contains(&q)
                || item
                    .process
                    .ports
                    .iter()
                    .any(|port| port.to_string().contains(&q))
        });
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_proc(pid: u32, ppid: Option<u32>, name: &str, cpu: f32) -> ProcessHarvest {
        ProcessHarvest {
            pid,
            parent_pid: ppid,
            name: name.to_string(),
            cpu_usage: cpu,
            memory_bytes: 1024 * 1024 * 100,
            virtual_memory_bytes: 1024 * 1024 * 500,
            memory_percent: 5.0,
            status: "Run".to_string(),
            cmd: format!("{} --dev", name),
            exe: format!("/usr/bin/{}", name),
            cwd: "/home/dev".to_string(),
            user: "axyl".to_string(),
            run_time_secs: 300,
            ports: vec![3000],
            children: vec![],
        }
    }

    #[test]
    fn test_build_process_tree_hierarchy() {
        // PID 1 (systemd) -> PID 100 (node) -> PID 200 (worker1), PID 201 (worker2)
        let procs = vec![
            dummy_proc(1, None, "systemd", 1.0),
            dummy_proc(100, Some(1), "node", 20.0),
            dummy_proc(200, Some(100), "worker1", 10.0),
            dummy_proc(201, Some(100), "worker2", 5.0),
        ];

        let tree = build_process_tree(&procs, "");
        assert_eq!(tree.len(), 4);

        assert_eq!(tree[0].process.pid, 1);
        assert_eq!(tree[0].depth, 0);
        assert_eq!(tree[0].prefix, "");

        assert_eq!(tree[1].process.pid, 100);
        assert_eq!(tree[1].depth, 1);
        assert_eq!(tree[1].prefix, "└─ ");

        assert_eq!(tree[2].process.pid, 200);
        assert_eq!(tree[2].depth, 2);
        assert_eq!(tree[2].prefix, "   ├─ ");

        assert_eq!(tree[3].process.pid, 201);
        assert_eq!(tree[3].depth, 2);
        assert_eq!(tree[3].prefix, "   └─ ");
    }

    #[test]
    fn test_build_process_tree_orphans_and_search() {
        // PPID 999 does not exist in procs -> PID 50 becomes root
        let procs = vec![
            dummy_proc(50, Some(999), "vite", 15.0),
            dummy_proc(60, Some(50), "esbuild", 2.0),
        ];

        let tree = build_process_tree(&procs, "");
        assert_eq!(tree.len(), 2);
        assert_eq!(tree[0].process.pid, 50);
        assert_eq!(tree[0].depth, 0);

        // Search query filtering
        let filtered = build_process_tree(&procs, "esbuild");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].process.name, "esbuild");

        let port_filtered = build_process_tree(&procs, "3000");
        assert_eq!(port_filtered.len(), 2);
    }
}
