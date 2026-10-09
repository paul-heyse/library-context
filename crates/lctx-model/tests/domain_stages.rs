use lctx_model::domain::{input::*, stages::*, *};
fn stage_model() -> ValidatedModel {
    ValidatedModel::declared(vec![Relation::of::<Package>(), Relation::of::<Release>()]).unwrap()
}
fn stages() -> Vec<Stage> {
    vec![
        Stage {
            captured_binding: None,
name: "packages",
            inputs: vec![],
            outputs: vec![RelationUse::of::<Package>()],
            contributes: vec![],
            coverage: vec![],
            profiles: vec![Profile::Catalog, Profile::Behavioral],
            effect: Effect::Pure,
            code: ContentHash::of(b"test-producer"),
            configuration: ContentHash::of(b"test-config"),
        },
        Stage {
            captured_binding: None,
name: "releases",
            inputs: vec![RelationUse::of::<Package>()],
            outputs: vec![RelationUse::of::<Release>()],
            contributes: vec![],
            coverage: vec![],
            profiles: vec![Profile::Catalog, Profile::Behavioral],
            effect: Effect::Pure,
            code: ContentHash::of(b"test-producer"),
            configuration: ContentHash::of(b"test-config"),
        },
    ]
}

#[test]
fn producer_identity_effect_configuration_and_profile_are_schedule_contracts() {
    let model = stage_model();
    let digest = |stages, profile| {
        Schedule::build(&model, stages, &[], profile)
            .unwrap()
            .digest()
    };
    let original = digest(stages(), Profile::Catalog);
    for change in ["code", "configuration", "effect"] {
        let mut next = stages();
        match change {
            "code" => next[0].code = ContentHash::of(b"changed producer"),
            "configuration" => next[0].configuration = ContentHash::of(b"changed configuration"),
            _ => next[0].effect = Effect::Extraction,
        }
        assert_ne!(original, digest(next, Profile::Catalog), "{change}");
    }
    assert_ne!(original, digest(stages(), Profile::Behavioral));
    let mut selected = stages();
    selected[0].profiles = vec![Profile::Behavioral];
    assert!(
        Schedule::build(&model, selected.clone(), &[], Profile::Catalog).is_err(),
        "a skipped writer cannot supply an input"
    );
    assert!(Schedule::build(&model, selected, &[], Profile::Behavioral).is_ok());
    let mut invalid = stages();
    invalid[0].profiles.clear();
    assert!(Schedule::build(&model, invalid, &[], Profile::Catalog).is_err());
}

#[test]
fn the_schedule_refuses_double_or_missing_writers_foreign_relations_and_cycles() {
    let model = stage_model();
    let of = |name: &'static str,
              inputs: Vec<RelationUse>,
              outputs: Vec<RelationUse>,
              profiles: Vec<Profile>| Stage {
        captured_binding: None,
name,
        inputs,
        outputs,
        contributes: vec![],
        coverage: vec![],
        profiles,
        effect: Effect::Pure,
        code: ContentHash::of(b"test-producer"),
        configuration: ContentHash::of(b"test-config"),
    };
    let both = || vec![Profile::Catalog, Profile::Behavioral];
    let (package, release) = (RelationUse::of::<Package>, RelationUse::of::<Release>);
    let refused =
        |stages: Vec<Stage>, required: &[RelationUse], profile: Profile, expected: &str| {
            let error = Schedule::build(&model, stages, required, profile)
                .map(|_| ())
                .unwrap_err()
                .to_string();
            assert!(
                error.contains(expected),
                "expected `{expected}`, got `{error}`"
            );
        };
    refused(
        vec![
            of("a", vec![], vec![package()], both()),
            of("b", vec![], vec![package()], both()),
        ],
        &[],
        Profile::Catalog,
        "multiple writers",
    );
    // One writer per profile is not a double writer.
    let per_profile = || {
        vec![
            of(
                "behavior",
                vec![],
                vec![package()],
                vec![Profile::Behavioral],
            ),
            of("populate", vec![], vec![package()], vec![Profile::Catalog]),
        ]
    };
    for (profile, writer) in [
        (Profile::Catalog, "populate"),
        (Profile::Behavioral, "behavior"),
    ] {
        let schedule = Schedule::build(&model, per_profile(), &[package()], profile).unwrap();
        assert_eq!(
            schedule.stages().iter().map(|s| s.name).collect::<Vec<_>>(),
            [writer]
        );
    }
    refused(
        vec![of("a", vec![package()], vec![release()], both())],
        &[],
        Profile::Catalog,
        "missing writer for",
    );
    refused(
        vec![of("a", vec![], vec![package()], both())],
        &[release()],
        Profile::Catalog,
        "missing required writer",
    );
    refused(
        vec![of("a", vec![], vec![RelationUse::of::<Outside>()], both())],
        &[],
        Profile::Catalog,
        "undeclared relation",
    );
    refused(
        vec![of(
            "a",
            vec![RelationUse::of::<Outside>()],
            vec![package()],
            both(),
        )],
        &[],
        Profile::Catalog,
        "undeclared relation",
    );
    refused(
        vec![of("a", vec![package()], vec![package()], both())],
        &[],
        Profile::Catalog,
        "stage cycle",
    );
    refused(
        vec![
            of("a", vec![release()], vec![package()], both()),
            of("b", vec![package()], vec![release()], both()),
        ],
        &[],
        Profile::Catalog,
        "stage cycle",
    );
    refused(
        vec![
            of("a", vec![], vec![package()], both()),
            of("a", vec![], vec![release()], both()),
        ],
        &[],
        Profile::Catalog,
        "duplicate stage",
    );
    refused(
        vec![of("a", vec![], vec![package()], vec![])],
        &[],
        Profile::Catalog,
        "invalid profiles",
    );
    // A writer the profile does not request leaves its reader unscheduled, never reading an empty
    // relation as if it were complete.
    refused(
        vec![
            of("flow", vec![], vec![package()], vec![Profile::Behavioral]),
            of("reader", vec![package()], vec![release()], both()),
        ],
        &[],
        Profile::Catalog,
        "missing writer for",
    );
}

#[derive(Debug, Clone, PartialEq, Eq, lctx_model::Domain)]
#[model(name = "outside_the_model")]
struct Outside {
    #[model(key)]
    name: String,
}
