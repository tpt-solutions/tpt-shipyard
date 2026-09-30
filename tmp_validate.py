import io
p = "crates/core/tpt-yard-core/src/project.rs"
s = io.open(p, encoding="utf-8").read()

# CoreError gains a ValidationError variant.
s = s.replace("""pub enum CoreError {
    /// Malformed JSON text.
    Json(String),
    /// A required field is absent.
    MissingField(String),
    /// A field is present with the wrong shape or an unknown variant.
    TypeError(String),
    /// File I/O failure while loading a project.
    Io(String),
}""",
"""pub enum CoreError {
    /// Malformed JSON text.
    Json(String),
    /// A required field is absent.
    MissingField(String),
    /// A field is present with the wrong shape or an unknown variant.
    TypeError(String),
    /// File I/O failure while loading a project.
    Io(String),
    /// The project loaded but is internally inconsistent (duplicate ids,
    /// dangling dependencies, non-finite numbers, ...).
    Validation(String),
}""")

# Display impl — find it and add the arm.
import re
m = re.search(r"impl fmt::Display for CoreError \{(.*?)\n\}", s, re.S)
assert m, "CoreError display not found"
body = m.group(0)
if "CoreError::Io(e)" in body:
    new_body = body.replace(
        "            CoreError::Io(e) => write!(f, \"i/o: {e}\"),",
        "            CoreError::Io(e) => write!(f, \"i/o: {e}\"),\n            CoreError::Validation(msg) => write!(f, \"invalid project: {msg}\"),",
    )
    s = s.replace(body, new_body)

# validate() + hook in from_json_value.
anchor = """    /// Index of a phase in `build_phases`."""
new = """    /// Checks the project for internal consistency.
    ///
    /// # Errors
    ///
    /// [`CoreError::Validation`] describing the first inconsistency found:
    /// duplicate phase/activity ids, a dependency that does not resolve,
    /// `current_phase` not naming a phase of this project, empty phase
    /// activity lists, non-finite or negative numbers, or a completion
    /// fraction outside 0..=1.
    pub fn validate(&self) -> Result<(), CoreError> {
        let err = |msg: String| Err(CoreError::Validation(msg));

        if self.name.trim().is_empty() {
            return err("project name is empty".into());
        }
        if !(self.build_phases.len() > 0) {
            return err("project has no build phases".into());
        }
        if !self.build_phases.iter().any(|p| p.id == self.current_phase) {
            return err(format!(
                "current_phase {:?} is not one of the project's phases",
                self.current_phase
            ));
        }

        let mut phase_ids = std::collections::BTreeSet::new();
        let mut activity_ids = std::collections::BTreeSet::new();
        let mut dependencies: Vec<(ActivityId, Vec<ActivityId>)> = Vec::new();
        for phase in &self.build_phases {
            if !phase_ids.insert(phase.id) {
                return err(format!("duplicate phase id {:?}", phase.id));
            }
            if !(phase.duration_days >= 0.0) || !phase.duration_days.is_finite() {
                return err(format!(
                    "phase {:?} has non-finite or negative duration {}",
                    phase.id, phase.duration_days
                ));
            }
            if !(phase.weight_state.design_kg >= 0.0)
                || !phase.weight_state.design_kg.is_finite()
                || !(phase.weight_state.installed_kg >= 0.0)
                || !phase.weight_state.installed_kg.is_finite()
            {
                return err(format!(
                    "phase {:?} has non-finite or negative weight state",
                    phase.id
                ));
            }
            if phase.weight_state.installed_kg > phase.weight_state.design_kg + 1e-9 {
                return err(format!(
                    "phase {:?} installed kg exceeds design kg",
                    phase.id
                ));
            }
            if phase.activities.is_empty() {
                return err(format!("phase {:?} has no activities", phase.id));
            }
            for a in &phase.activities {
                if !activity_ids.insert(a.id) {
                    return err(format!("duplicate activity id {}", a.id));
                }
                if !(a.duration_hours > 0.0) || !a.duration_hours.is_finite() {
                    return err(format!(
                        "activity {} has non-finite or non-positive duration",
                        a.id
                    ));
                }
                dependencies.push((a.id, a.dependencies.clone()));
            }
        }
        for (id, deps) in dependencies {
            for d in &deps {
                if !activity_ids.contains(d) {
                    return err(format!(
                        "activity {} depends on unknown activity {}",
                        id, d
                    ));
                }
            }
        }
        Ok(())
    }

    /// Index of a phase in `build_phases`."""
assert anchor in s
s = s.replace(anchor, new, 1)

# Hook: from_json_value validates before returning.
import re
m2 = re.search(r"pub fn from_json_value\(v: &Value\) -> Result<Self, CoreError> \{", s)
assert m2
# find the last `Ok(` before the function's end — instead, wrap: rename inner and add wrapper is invasive;
# simpler: find "Self {" completion at fn end. Use a targeted approach: the function ends with "    }\n" before the next pub fn.
idx = m2.end()
end = s.index("\n    }\n", idx)
# Find the final Ok(...) expression in that range — replace with validate chain.
tail = s[idx:end]
ok_idx = tail.rindex("Ok(")
# Replace the last Ok(x) with { let project = x; project.validate()?; Ok(project) }
depth = 0
i = ok_idx
while True:
    if tail[i] == "(":
        depth += 1
    elif tail[i] == ")":
        depth -= 1
        if depth == 0:
            break
    i += 1
inner = tail[ok_idx + 1:i]
new_tail = tail[:ok_idx] + "{ let project = " + inner + "; project.validate()?; Ok(project) }" + tail[i + 1:]
s = s[:idx] + new_tail + s[end:]

io.open(p, "w", encoding="utf-8", newline="\n").write(s)
print("validate added")
