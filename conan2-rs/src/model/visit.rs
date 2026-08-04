use crate::model::{ConanRoot, Graph, ModelError, Recipe};

impl ConanRoot {
    pub fn get_recipe_by_name(&self, name: &str) -> Result<&Recipe, ModelError> {
        self.graph.get_recipe_by_name(name)
    }

    pub fn get_recipe_by_id(&self, id: &str) -> Result<&Recipe, ModelError> {
        self.graph.get_recipe_by_id(id)
    }

    pub fn get_cpp_info(&self, deps: &[&str]) -> Result<CppInfo, ModelError> {
        self.graph.get_cpp_info(deps)
    }
}

impl Graph {
    pub fn get_recipe_by_name(&self, name: &str) -> Result<&Recipe, ModelError> {
        let mut recipe = self.nodes.iter().filter_map(|(_id, recipe)| {
            if let Some(n) = &recipe.package_reference.name {
                if n == name { Some(recipe) } else { None }
            } else {
                None
            }
        });
        recipe
            .next()
            .ok_or(ModelError::RecipeNotFound(name.to_string()))
    }

    pub fn get_recipe_by_id(&self, id: &str) -> Result<&Recipe, ModelError> {
        self.nodes
            .get(id)
            .ok_or(ModelError::RecipeNotFound(id.to_string()))
    }

    pub fn get_cpp_info(&self, deps: &[&str]) -> Result<CppInfo, ModelError> {
        let mut cpp_info = CppInfo::default();
        for dep in deps.iter() {
            // split dep, format <package>::<componet>
            let parts = dep.split("::").collect::<Vec<_>>();
            if parts.len() == 2 {
                let dep_name = parts[0];
                let mut dep_comp = parts[1];
                if dep_comp == dep_name {
                    dep_comp = "root";
                }
                let dep = self.get_recipe_by_name(dep_name)?;
                self.update_cpp_info(dep, dep_comp, &mut cpp_info)?;
            } else {
                return Err(ModelError::InvalidDependency(dep.to_string()));
            }
        }
        Ok(cpp_info)
    }

    fn update_cpp_info(
        &self,
        recipe: &Recipe,
        comp: &str,
        cpp_info: &mut CppInfo,
    ) -> Result<(), ModelError> {
        let comp = recipe
            .consumer_info
            .cpp_info
            .get(comp)
            .ok_or(ModelError::InvalidDependency(format!(
                "{:?}:{}",
                &recipe.package_reference.name, comp
            )))?;

        comp.includedirs
            .clone()
            .map(|dirs| append_to_vec(&mut cpp_info.include_dirs, &dirs));
        comp.libdirs
            .clone()
            .map(|dirs| append_to_vec(&mut cpp_info.lib_dirs, &dirs));
        comp.libs
            .clone()
            .map(|libs| append_to_vec(&mut cpp_info.libs, &libs));

        if let Some(requires) = &comp.requires {
            for req in requires.iter() {
                let parts = req.split("::").collect::<Vec<_>>();
                if parts.len() == 2 {
                    let dep_name = parts[0];
                    let mut dep_comp = parts[1];
                    if dep_comp == dep_name {
                        dep_comp = "root";
                    }
                    let dep = self.get_recipe_by_name(dep_name)?;
                    self.update_cpp_info(dep, dep_comp, cpp_info)?;
                } else if parts.len() == 1 {
                    self.update_cpp_info(recipe, req, cpp_info)?;
                }
            }
        }
        Ok(())
    }
}

fn append_to_vec(target: &mut Vec<String>, src: &[String]) {
    for s in src.iter() {
        if !target.contains(s) {
            target.push(s.clone());
        }
    }
}

#[derive(Debug, Default)]
pub struct CppInfo {
    pub include_dirs: Vec<String>,
    pub lib_dirs: Vec<String>,
    pub libs: Vec<String>,
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    #[test]
    fn test_hyperscan() -> anyhow::Result<()> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")?;
        let path = std::path::Path::new(&manifest_dir).join("tests/windows-hyperscan.json");
        let data = fs::read_to_string(&path)?;
        let root: ConanRoot = serde_json::from_str(&data)?;

        let cpp = root.get_cpp_info(&["hyperscan::hs", "hyperscan::hs_runtime"])?;
        println!("cpp info: {:?}", cpp);

        let cpp = root.get_cpp_info(&["hyperscan::hs"])?;
        println!("cpp info: {:?}", cpp);

        Ok(())
    }
}
