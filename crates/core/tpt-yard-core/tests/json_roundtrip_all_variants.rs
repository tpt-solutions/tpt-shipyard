//! Review 7D: every enum variant of the wire format must survive a JSON
//! round-trip exactly — a variant that serializes but cannot load back (or
//! silently loads as a different variant) is a wire-format bug.
//!
//! The variants are enumerated explicitly rather than via a macro so adding
//! a new variant without wiring it through the JSON format shows up here as
//! a count mismatch or a failing match.

use tpt_yard_assembly::ActivityId;
use tpt_yard_core::{
    ActivityType, BuildPhase, ConstructionMethod, FluidType, HullType, OutfitSystem, PhaseId,
    ProjectId, PropulsionType, Resource, ResourceKind, SeaVesselType, SpaceVesselType,
    StructuralState, TestType, VesselProject, VesselType, WeightState,
};

/// A one-phase project whose activity carries `activity`.
fn project_with(activity: ActivityType) -> VesselProject {
    let mut phase = BuildPhase::new(PhaseId(1), "P1", 3.0);
    let a = tpt_yard_core::AssemblyActivity::new(ActivityId(1), "a", activity, 4.0).with_resources(
        vec![Resource {
            name: "Goliath crane".into(),
            kind: ResourceKind::Crane,
            capacity: 800.0,
        }],
    );
    phase.activities.push(a);
    phase.weight_state = WeightState {
        design_kg: 100.0,
        installed_kg: 0.0,
    };
    VesselProject::new(
        ProjectId(1),
        "round trip",
        VesselType::Sea(SeaVesselType::FishingVessel),
        ConstructionMethod::SeaDrydock,
        vec![phase],
    )
    .unwrap()
}

fn round_trips(project: &VesselProject) {
    let json = project.to_json().to_string_compact();
    let back = VesselProject::from_json_str(&json).expect("loads back");
    assert_eq!(&back, project, "round-trip mismatch");
    let pretty = project.to_json().to_string_pretty();
    let back = VesselProject::from_json_str(&pretty).expect("loads back");
    assert_eq!(&back, project, "pretty round-trip mismatch");
}

#[test]
fn every_vessel_type_variant_round_trips() {
    let sea: Vec<SeaVesselType> = vec![
        SeaVesselType::ContainerShip {
            teu_capacity: 14_000,
        },
        SeaVesselType::Tanker {
            deadweight_tonnes: 105_000.5,
        },
        SeaVesselType::LngCarrier {
            cargo_volume_m3: 174_000.0,
        },
        SeaVesselType::CruiseShip { passengers: 5_400 },
        SeaVesselType::NavalVessel {
            classification: "frigate".into(),
        },
        SeaVesselType::Submarine {
            hull_type: HullType::SingleHull,
        },
        SeaVesselType::Submarine {
            hull_type: HullType::DoubleHull,
        },
        SeaVesselType::OffshoreVessel {
            vessel_class: "PSV".into(),
        },
        SeaVesselType::FishingVessel,
    ];
    let space: Vec<SpaceVesselType> = vec![
        SpaceVesselType::SpaceStation { modules: 8 },
        SpaceVesselType::OrbitalHabitat {
            rotation_rpm: 2.0,
            radius_m: 100.0,
        },
        SpaceVesselType::SolarPowerStation {
            array_area_m2: 25_000.0,
        },
        SpaceVesselType::DeepSpaceVessel {
            propulsion: PropulsionType::ChemicalBiPropellant,
        },
        SpaceVesselType::DeepSpaceVessel {
            propulsion: PropulsionType::ChemicalMonoPropellant,
        },
        SpaceVesselType::DeepSpaceVessel {
            propulsion: PropulsionType::Solid,
        },
        SpaceVesselType::DeepSpaceVessel {
            propulsion: PropulsionType::NuclearThermal,
        },
        SpaceVesselType::DeepSpaceVessel {
            propulsion: PropulsionType::NuclearElectric,
        },
        SpaceVesselType::DeepSpaceVessel {
            propulsion: PropulsionType::SolarElectric,
        },
        SpaceVesselType::DeepSpaceVessel {
            propulsion: PropulsionType::SolarSail,
        },
        SpaceVesselType::LunarVehicle,
        SpaceVesselType::MarsVehicle,
        SpaceVesselType::OrbitalFactory,
        SpaceVesselType::FuelDepot,
    ];

    let mut checked = 0;
    for v in &sea {
        let mut p = project_with(ActivityType::CutSteel);
        p.vessel_type = VesselType::Sea(v.clone());
        round_trips(&p);
        checked += 1;
    }
    for v in &space {
        let mut p = project_with(ActivityType::CutSteel);
        p.vessel_type = VesselType::Space(v.clone());
        round_trips(&p);
        checked += 1;
    }
    let mut p = project_with(ActivityType::CutSteel);
    p.vessel_type = VesselType::Hybrid;
    round_trips(&p);
    checked += 1;
    assert_eq!(checked, 9 + 14 + 1, "variant coverage drifted");
}

#[test]
fn every_construction_method_variant_round_trips() {
    for m in [
        ConstructionMethod::SeaBlockConstruction,
        ConstructionMethod::SeaSlipwayLaunch,
        ConstructionMethod::SeaDrydock,
        ConstructionMethod::OrbitalAssembly,
        ConstructionMethod::InSpaceManufacturing,
        ConstructionMethod::LunarSurfaceConstruction,
    ] {
        let mut p = project_with(ActivityType::CutSteel);
        p.construction_method = m;
        round_trips(&p);
    }
}

#[test]
fn every_activity_type_variant_round_trips() {
    // All 13 ActivityType variants; the Outfit and Test payloads cover all
    // 6 OutfitSystem and all 7 TestType variants respectively.
    let variants: Vec<ActivityType> = vec![
        ActivityType::CutSteel,
        ActivityType::FormPlate,
        ActivityType::WeldBlock,
        ActivityType::LiftBlock,
        ActivityType::JoinBlock,
        ActivityType::Outfit {
            system: OutfitSystem::Piping {
                fluid: FluidType::Cryogenic,
                diameter_mm: 150.0,
            },
        },
        ActivityType::Outfit {
            system: OutfitSystem::Electrical {
                voltage_v: 6900.0,
                cable_type: "MV-85".into(),
            },
        },
        ActivityType::Outfit {
            system: OutfitSystem::Hvac {
                duct_size_mm: 400.0,
            },
        },
        ActivityType::Outfit {
            system: OutfitSystem::Structural {
                member_type: "T-bar".into(),
            },
        },
        ActivityType::Outfit {
            system: OutfitSystem::Machinery {
                equipment: "thruster".into(),
            },
        },
        ActivityType::Outfit {
            system: OutfitSystem::Navigation {
                equipment: "radar".into(),
            },
        },
        ActivityType::Paint,
        ActivityType::Test {
            test_type: TestType::Hydrostatic,
        },
        ActivityType::Test {
            test_type: TestType::StrengthLoad,
        },
        ActivityType::Test {
            test_type: TestType::Tightness,
        },
        ActivityType::Test {
            test_type: TestType::Ndt,
        },
        ActivityType::Test {
            test_type: TestType::StabilityInclining,
        },
        ActivityType::Test {
            test_type: TestType::SeaTrial,
        },
        ActivityType::Test {
            test_type: TestType::Pressure,
        },
        ActivityType::Launch,
        ActivityType::Dock,
        ActivityType::RoboticAssembly {
            robot: tpt_yard_core::RobotId(1),
        },
        ActivityType::AdditiveManufacturing {
            material: "AA5083 wire".into(),
        },
        ActivityType::PropellantLoad,
    ];
    assert_eq!(
        variants.len(),
        24,
        "an ActivityType variant was added — extend this list and the JSON format together"
    );

    for v in variants {
        round_trips(&project_with(v));
    }
}

#[test]
fn every_structural_state_variant_round_trips() {
    for s in [
        StructuralState::NotStarted,
        StructuralState::Partial { completion: 0.42 },
        StructuralState::Closed,
        StructuralState::StrengthVerified,
    ] {
        let mut p = project_with(ActivityType::CutSteel);
        p.build_phases[0].structural_state = s;
        round_trips(&p);
    }
}

#[test]
fn every_resource_kind_variant_round_trips() {
    for kind in [
        ResourceKind::Crane,
        ResourceKind::Workshop,
        ResourceKind::Drydock,
        ResourceKind::WeldingStation,
        ResourceKind::Robot,
        ResourceKind::Crew,
        ResourceKind::Transport,
    ] {
        let p = project_with(ActivityType::LiftBlock)
            .to_json()
            .to_string_compact();
        let mut p = VesselProject::from_json_str(&p).unwrap();
        p.build_phases[0].activities[0].resources = vec![Resource {
            name: "res".into(),
            kind,
            capacity: 1.0,
        }];
        round_trips(&p);
    }
}
