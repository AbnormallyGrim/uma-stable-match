use super::*;

fn guild(roles: &[(&str, &str)]) -> Guild {
    Guild {
        owner_id: "1".into(),
        roles: roles
            .iter()
            .map(|(id, permissions)| Role {
                id: (*id).into(),
                name: format!("role {id}"),
                permissions: (*permissions).into(),
            })
            .collect(),
    }
}

fn member(roles: &[&str]) -> Member {
    Member {
        roles: roles.iter().map(|r| (*r).to_owned()).collect(),
    }
}

#[test]
fn the_owner_manages() {
    assert!(has_manage(&guild(&[]), 10, 1, &member(&[])));
}

#[test]
fn administrator_or_manage_server_in_a_role_manages() {
    let guild = guild(&[("10", "0"), ("20", "8"), ("21", "32"), ("22", "1024")]);
    assert!(has_manage(&guild, 10, 2, &member(&["20"])));
    assert!(has_manage(&guild, 10, 2, &member(&["21"])));
    assert!(!has_manage(&guild, 10, 2, &member(&["22"])));
    assert!(!has_manage(&guild, 10, 2, &member(&[])));
}

#[test]
fn the_everyone_role_counts_and_roles_the_member_lacks_do_not() {
    let granted = guild(&[("10", "32"), ("20", "0")]);
    assert!(has_manage(&granted, 10, 2, &member(&[])));
    let not_theirs = guild(&[("10", "0"), ("20", "8")]);
    assert!(!has_manage(&not_theirs, 10, 2, &member(&["30"])));
}

#[test]
fn permissions_combine_across_roles_and_junk_is_ignored() {
    let guild = guild(&[
        ("10", "0"),
        ("20", "16"),
        ("21", "not a number"),
        ("22", "32"),
    ]);
    assert!(has_manage(&guild, 10, 2, &member(&["20", "21", "22"])));
    assert!(!has_manage(&guild, 10, 2, &member(&["20", "21"])));
}

fn rights(manage: bool, held: &[u64]) -> Rights {
    Rights {
        manage,
        held: held.to_vec(),
        roles: Vec::new(),
    }
}

#[test]
fn managing_the_server_allows_everything() {
    assert_eq!(rights(true, &[]).level(&[]), Some(Level::Manage));
    assert_eq!(rights(true, &[5]).level(&[5, 6]), Some(Level::Manage));
}

#[test]
fn a_leader_role_allows_viewing_only_that_clubs_applications() {
    assert_eq!(rights(false, &[5]).level(&[5, 6]), Some(Level::View));
    assert_eq!(rights(false, &[6]).level(&[5, 6]), Some(Level::View));
    assert_eq!(rights(false, &[7]).level(&[5, 6]), None);
    assert_eq!(rights(false, &[5]).level(&[]), None);
    assert_eq!(rights(false, &[]).level(&[5]), None);
}

#[test]
fn levels_have_names() {
    assert_eq!(Level::View.as_str(), "view");
    assert_eq!(Level::Manage.as_str(), "manage");
}
