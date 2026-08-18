use crate::core::detector::{DeveloperMeta, is_dev_port};
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
    /// Developer classification metadata (framework, runtime, category, project name).
    pub dev_meta: DeveloperMeta,
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

fn primary_port(p: &ProcessHarvest) -> Option<u16> {
    p.ports.iter().copied().filter(|&p| is_dev_port(p)).min()
}

fn pick_dev_server_repr<'a>(members: &mut [&'a ProcessHarvest]) -> (&'a ProcessHarvest, usize) {
    // Stable pick: listener over portless, then lowest PID. Never CPU — that jitters every tick.
    members.sort_by(|a, b| {
        primary_port(b)
            .is_some()
            .cmp(&primary_port(a).is_some())
            .then_with(|| a.pid.cmp(&b.pid))
    });
    (members[0], members.len().saturating_sub(1))
}

fn same_project(a: &ProcessHarvest, b: &ProcessHarvest) -> bool {
    !a.cwd.is_empty() && a.cwd != "-" && a.cwd == b.cwd
}

/// Collapse workers / parent CLI into one row per listening server.
///
/// Grouping is by **port first**. Portless processes (e.g. `next dev` before bind,
/// or the parent while a child holds :3000) attach only when there is exactly one
/// matching listener with the same framework + project. Two apps on :3000 and :3001
/// in the same directory stay as two rows.
pub fn collapse_dev_servers<'a>(
    candidates: impl IntoIterator<Item = &'a ProcessHarvest>,
) -> Vec<(&'a ProcessHarvest, usize)> {
    let mut by_port: HashMap<(String, u16), Vec<&'a ProcessHarvest>> = HashMap::new();
    let mut portless: Vec<&'a ProcessHarvest> = Vec::new();

    for p in candidates {
        if let Some(port) = primary_port(p) {
            let fw = p.dev_meta.framework.label().to_string();
            by_port.entry((fw, port)).or_default().push(p);
        } else {
            portless.push(p);
        }
    }

    let mut unattached: Vec<&'a ProcessHarvest> = Vec::new();
    for p in portless {
        let fw = p.dev_meta.framework.label();
        let mut matches: Vec<(String, u16)> = by_port
            .iter()
            .filter_map(|((gfw, port), members)| {
                if gfw == fw && members.iter().any(|m| same_project(m, p)) {
                    Some((gfw.clone(), *port))
                } else {
                    None
                }
            })
            .collect();
        matches.sort_unstable();
        matches.dedup();

        if matches.len() == 1 {
            by_port
                .get_mut(&matches[0])
                .expect("key from by_port")
                .push(p);
        } else {
            unattached.push(p);
        }
    }

    let mut cwd_groups: HashMap<String, Vec<&'a ProcessHarvest>> = HashMap::new();
    for p in unattached {
        let fw = p.dev_meta.framework.label();
        let key = if !p.cwd.is_empty() && p.cwd != "-" {
            format!("{fw}:cwd:{}", p.cwd)
        } else if let Some(name) = p.dev_meta.project_name.as_deref().filter(|s| !s.is_empty()) {
            format!("{fw}:proj:{name}")
        } else {
            format!("{fw}:pid:{}", p.pid)
        };
        cwd_groups.entry(key).or_default().push(p);
    }

    let mut collapsed: Vec<(&'a ProcessHarvest, usize)> = by_port
        .into_values()
        .chain(cwd_groups.into_values())
        .map(|mut members| pick_dev_server_repr(&mut members))
        .collect();

    collapsed.sort_by(|(a, _), (b, _)| match (primary_port(a), primary_port(b)) {
        (Some(pa), Some(pb)) => pa.cmp(&pb).then_with(|| a.pid.cmp(&b.pid)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => {
            let a_name = a.dev_meta.project_name.as_deref().unwrap_or(&a.cwd);
            let b_name = b.dev_meta.project_name.as_deref().unwrap_or(&b.cwd);
            a_name
                .cmp(b_name)
                .then_with(|| a.cwd.cmp(&b.cwd))
                .then_with(|| a.pid.cmp(&b.pid))
        }
    });

    collapsed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::detector::{FrameworkType, ProcessCategory};

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
            dev_meta: DeveloperMeta::unknown(),
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

    fn next_proc(pid: u32, cwd: &str, ports: Vec<u16>, cpu: f32) -> ProcessHarvest {
        let mut p = dummy_proc(pid, Some(1), "node", cpu);
        p.cwd = cwd.to_string();
        p.ports = ports;
        p.dev_meta = DeveloperMeta {
            runtime: crate::core::detector::RuntimeType::Node,
            framework: FrameworkType::NextJs,
            category: if p.ports.is_empty() {
                ProcessCategory::RuntimeProcess
            } else {
                ProcessCategory::DevServer
            },
            project_name: Some("my-app".to_string()),
            dev_url: p
                .ports
                .first()
                .map(|port| format!("http://localhost:{port}")),
        };
        p
    }

    #[test]
    fn test_collapse_nextjs_workers_same_cwd() {
        let procs = vec![
            next_proc(100, "/home/dev/my-app", vec![3000], 1.0),
            next_proc(101, "/home/dev/my-app", vec![], 0.2),
            next_proc(102, "/home/dev/my-app", vec![], 0.1),
            next_proc(103, "/home/dev/my-app", vec![], 0.0),
        ];

        let collapsed = collapse_dev_servers(&procs);
        assert_eq!(collapsed.len(), 1);
        assert_eq!(collapsed[0].0.pid, 100);
        assert_eq!(collapsed[0].0.ports, vec![3000]);
        assert_eq!(collapsed[0].1, 3);
    }

    #[test]
    fn test_collapse_keeps_two_ports_in_same_cwd() {
        let procs = vec![
            next_proc(100, "/home/dev/my-app", vec![3000], 1.0),
            next_proc(200, "/home/dev/my-app", vec![3001], 1.0),
        ];

        let collapsed = collapse_dev_servers(&procs);
        assert_eq!(collapsed.len(), 2);
        let ports: Vec<u16> = collapsed
            .iter()
            .filter_map(|(p, _)| p.ports.first().copied())
            .collect();
        assert!(ports.contains(&3000));
        assert!(ports.contains(&3001));
    }

    #[test]
    fn test_collapse_keeps_distinct_projects() {
        let procs = vec![
            next_proc(100, "/home/dev/app-a", vec![3000], 1.0),
            next_proc(200, "/home/dev/app-b", vec![3001], 1.0),
        ];

        let collapsed = collapse_dev_servers(&procs);
        assert_eq!(collapsed.len(), 2);
        let pids: Vec<u32> = collapsed.iter().map(|(p, _)| p.pid).collect();
        assert!(pids.contains(&100));
        assert!(pids.contains(&200));
    }

    #[test]
    fn test_collapse_attaches_portless_parent_to_single_listener() {
        let mut parent = next_proc(50, "/home/dev/my-app", vec![], 0.1);
        parent.dev_meta.category = ProcessCategory::DevServer;
        let procs = vec![next_proc(100, "/home/dev/my-app", vec![3000], 1.0), parent];

        let collapsed = collapse_dev_servers(&procs);
        assert_eq!(collapsed.len(), 1);
        assert_eq!(collapsed[0].0.pid, 100);
        assert_eq!(collapsed[0].1, 1);
    }

    #[test]
    fn test_collapse_order_stable_when_cpu_changes() {
        let high_cpu_second = vec![
            next_proc(200, "/home/dev/app-b", vec![3001], 90.0),
            next_proc(100, "/home/dev/app-a", vec![3000], 1.0),
        ];
        let high_cpu_first = vec![
            next_proc(100, "/home/dev/app-a", vec![3000], 90.0),
            next_proc(200, "/home/dev/app-b", vec![3001], 1.0),
        ];

        let a = collapse_dev_servers(&high_cpu_second);
        let b = collapse_dev_servers(&high_cpu_first);
        assert_eq!(a.len(), 2);
        assert_eq!(a[0].0.ports[0], 3000);
        assert_eq!(a[1].0.ports[0], 3001);
        assert_eq!(a[0].0.pid, b[0].0.pid);
        assert_eq!(a[1].0.pid, b[1].0.pid);
    }
}
