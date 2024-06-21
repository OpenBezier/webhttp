use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, PartialOrd)]
pub struct RpItem {
    pub eng: String,
    pub chn: String,
    pub enabled: bool, // Action is activated or not
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct RpAction {
    pub actions: HashMap<String, Vec<RpItem>>, // key is Page, Value is Multi-Action
}

/// Example
///
/// '''toml
///
///     name = "testapp"
///
///     [role]
///     default = []
///     admin = ["admin"]
///
///     [permission."user"]
///     "usermgt-xxx" = "default-false:admin-true"
///     "userread-xxx" = "default-true:admin-true"
/// '''
///
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct RpConfig {
    pub name: String,
    pub role: RpInputRole,
    pub permission: RpInputPermission,
}

// const ROLE_INTERNAL: &str = "__role_internal__";
pub type RpInputRole = HashMap<String, Vec<String>>;
pub type RpInputPermission = HashMap<String, HashMap<String, String>>;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct RpGroup {
    pub input_permission: RpInputPermission,
    pub input_role: RpInputRole,
    // Kye is role name, value.0 is permision list, value.1 is account list
    pub roles: HashMap<String, (RpAction, Vec<String>)>,
}

impl RpGroup {
    pub fn get_role(&self) -> HashMap<String, Vec<String>> {
        let mut role_map = HashMap::<String, Vec<String>>::default();
        for each in self.roles.iter() {
            role_map.insert(each.0.clone(), each.1 .1.clone());
        }
        role_map
    }

    pub fn get_user_permission(&self, user_account: String) -> HashMap<String, RpAction> {
        let mut user_permision = HashMap::<String, RpAction>::default();
        for (each_role, (actions, users)) in self.roles.iter() {
            if users.contains(&user_account) {
                user_permision.insert(each_role.clone(), actions.clone());
            }
        }

        if !user_permision.contains_key("default") {
            if let Some(default_actions) = self.roles.get("default") {
                user_permision.insert("default".into(), default_actions.0.clone());
            } else {
                tracing::warn!("default permission is not existed");
            }
        }
        user_permision
    }

    fn check_user_action_internal(actions: RpAction, page: String, item: String) -> bool {
        if let Some(action) = actions.actions.get(&page) {
            for each in action.iter() {
                if each.eng.eq(&item) && each.enabled {
                    return true;
                }
            }
        }
        false
    }
    pub fn check_user_action(&self, user_account: String, page: String, action: String) -> bool {
        let user_actions = self.get_user_permission(user_account.clone());
        for (_role_tmp, role_action) in user_actions.iter() {
            let check_status =
                Self::check_user_action_internal(role_action.clone(), page.clone(), action.clone());
            if check_status {
                return true;
            }
        }
        false
    }

    pub fn create(config: RpConfig) -> anyhow::Result<Self> {
        let role = config.role.clone();
        let permission = config.permission.clone();
        let mut role_permission = RpGroup::default();
        role_permission.input_permission = permission.clone();
        role_permission.input_role = role.clone();

        for (each_role, role_users) in role.iter() {
            let mut each_group_permisson = RpAction::default();
            let mut role_internal_group_permission = RpAction::default();

            for (each_group, action_map) in permission.iter() {
                for (each_action, action_info) in action_map.iter() {
                    if !each_action.contains("-") {
                        return Err(anyhow::anyhow!(
                            "action key is not splited by -, need eng and chn name: {}",
                            each_action
                        ));
                    }
                    let each_aciton_split: Vec<&str> = each_action.split("-").collect();
                    if each_aciton_split.len() != 2 {
                        return Err(anyhow::anyhow!(
                            "action key length is not 2 after splitting by -: {}",
                            each_action
                        ));
                    }
                    let aciton_eng = each_aciton_split.get(0).unwrap().to_string();
                    let aciton_chn = each_aciton_split.get(1).unwrap().to_string();

                    let mut action_info_list = HashMap::<String, bool>::default();
                    let action_info_split: Vec<&str> = action_info.split(":").collect();
                    for action_info_each in action_info_split.iter() {
                        let action_info_each = action_info_each.to_string();
                        if !action_info_each.contains("-") {
                            return Err(anyhow::anyhow!(
                                "action role value is not splited by -, need role and status: {}",
                                action_info
                            ));
                        }
                        let action_info_each_split: Vec<&str> =
                            action_info_each.split("-").collect();
                        if action_info_each_split.len() != 2 {
                            return Err(anyhow::anyhow!(
                                "action role value length is not 2 after splitting by -: {}",
                                action_info
                            ));
                        }

                        let action_role = action_info_each_split.get(0).unwrap().to_string();
                        let action_status = action_info_each_split.get(1).unwrap().to_string();
                        let action_status = match action_status.as_str() {
                            "true" => true,
                            "false" => false,
                            _ => false,
                        };
                        action_info_list.insert(action_role, action_status);
                    }

                    let action_item = if action_info_list.contains_key(each_role) {
                        RpItem {
                            eng: aciton_eng.clone(),
                            chn: aciton_chn.clone(),
                            enabled: action_info_list.get(each_role).unwrap().clone(),
                        }
                    } else {
                        RpItem {
                            eng: aciton_eng.clone(),
                            chn: aciton_chn.clone(),
                            enabled: false,
                        }
                    };
                    if each_group_permisson.actions.contains_key(each_group) {
                        let key_index = each_group_permisson.actions.get_mut(each_group).unwrap();
                        key_index.push(action_item);
                    } else {
                        each_group_permisson
                            .actions
                            .insert(each_group.clone(), vec![action_item]);
                    }

                    if role_internal_group_permission
                        .actions
                        .contains_key(each_group)
                    {
                        let key_index = role_internal_group_permission
                            .actions
                            .get_mut(each_group)
                            .unwrap();
                        key_index.push(RpItem {
                            eng: aciton_eng.clone(),
                            chn: aciton_chn.clone(),
                            enabled: false,
                        });
                    } else {
                        role_internal_group_permission.actions.insert(
                            each_group.clone(),
                            vec![RpItem {
                                eng: aciton_eng.clone(),
                                chn: aciton_chn.clone(),
                                enabled: false,
                            }],
                        );
                    }
                }
            }
            role_permission.roles.insert(
                each_role.clone(),
                (each_group_permisson.clone(), role_users.clone()),
            );

            // role_permission.roles.insert(
            //     ROLE_INTERNAL.to_string(),
            //     (role_internal_group_permission.clone(), vec![]),
            // );
        }
        Ok(role_permission)
    }
}
