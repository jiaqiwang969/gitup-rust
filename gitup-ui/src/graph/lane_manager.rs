/// Simple lane manager used to assign commits to lanes and advance to their parents.
/// Strategy:
/// - assign_lane: reuse lane if current id is already active; else first empty; else append new
/// - post_commit_update: current lane becomes first parent; remaining parents occupy nearest empty lanes to the right
pub struct LaneManager {
    active: Vec<Option<String>>, // per-lane active commit id expected next
}

impl LaneManager {
    pub fn new() -> Self { Self { active: Vec::new() } }

    /// Assign a lane for the given commit id
    pub fn assign_lane(&mut self, id: &str) -> usize {
        // 1) continue existing lane if id is already active
        for (i, slot) in self.active.iter().enumerate() {
            if slot.as_ref().map(|s| s == id).unwrap_or(false) {
                return i;
            }
        }

        // 2) use first empty slot
        for (i, slot) in self.active.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(id.to_string());
                return i;
            }
        }

        // 3) append new lane
        let idx = self.active.len();
        self.active.push(Some(id.to_string()));
        idx
    }

    /// After processing a commit at `current_lane`, advance lanes to its parents
    pub fn post_commit_update(&mut self, current_lane: usize, parents: &[String]) {
        if parents.is_empty() {
            if let Some(slot) = self.active.get_mut(current_lane) {
                *slot = None;
            }
            return;
        }
        // First parent continues in current lane
        if let Some(p0) = parents.first() {
            if let Some(slot) = self.active.get_mut(current_lane) {
                *slot = Some(p0.clone());
            }
        }
        // Assign remaining parents to nearest empty lanes to the right; create lanes as needed
        for p in parents.iter().skip(1) {
            let mut placed = false;
            for idx in current_lane+1..self.active.len() {
                if self.active[idx].is_none() {
                    self.active[idx] = Some(p.clone());
                    placed = true;
                    break;
                }
            }
            if !placed {
                self.active.push(Some(p.clone()));
            }
        }
    }

    pub fn lane_count(&self) -> usize { self.active.len() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_history_reuses_lane() {
        let mut lm = LaneManager::new();
        // HEAD commit B with parent A
        let lane_b = lm.assign_lane("B");
        assert_eq!(lane_b, 0);
        lm.post_commit_update(lane_b, &["A".to_string()]);
        // Now A should reuse lane 0
        let lane_a = lm.assign_lane("A");
        assert_eq!(lane_a, 0);
    }

    #[test]
    fn merge_creates_second_lane() {
        let mut lm = LaneManager::new();
        // Merge M with parents A (main) and F (feature)
        let lane_m = lm.assign_lane("M");
        assert_eq!(lane_m, 0);
        lm.post_commit_update(lane_m, &["A".to_string(), "F".to_string()]);
        // A should be lane 0, F should get lane 1
        let lane_a = lm.assign_lane("A");
        assert_eq!(lane_a, 0);
        let lane_f = lm.assign_lane("F");
        assert_eq!(lane_f, 1);
    }
}

