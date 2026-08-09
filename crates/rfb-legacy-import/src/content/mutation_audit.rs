// SPDX-License-Identifier: MPL-2.0

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use serde::{Deserialize, Serialize};

use super::{read_legacy_object_at, resolve_legacy_content_commit};
use crate::LegacyImportError;

const MUTATION_COUNT: usize = 152;
const MUTATION_SOURCE_PATHS: [&str; 6] = [
    "src/mut_a.c",
    "src/spells_a.c",
    "src/spells_c.c",
    "src/spells_h.c",
    "src/spells_m.c",
    "src/spells_s.c",
];
const GLOBAL_RULES: [&str; 7] = [
    "gain:class-berserker+type-activation=>weight-0",
    "gain:race-water-elemental+mutation-sp-to-hp-or-hp-to-sp=>weight-0",
    "gain:object-monster-body+organic-mutation=>weight-0",
    "gain:good-luck+bad-or-awful-rating=>weight-1",
    "gain:bad-luck-or-fragile+good-or-great-rating=>weight-1",
    "lose:good-luck+good-or-great-rating=>weight-1",
    "lose:bad-luck+bad-or-awful-rating=>weight-1",
];

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum MutationRating {
    Awful,
    Bad,
    Average,
    Good,
    Great,
}

impl MutationRating {
    fn parse(value: &str) -> Option<Self> {
        match value
            .trim()
            .trim_start_matches('{')
            .strip_prefix("MUT_RATING_")?
        {
            "AWFUL" => Some(Self::Awful),
            "BAD" => Some(Self::Bad),
            "AVERAGE" => Some(Self::Average),
            "GOOD" => Some(Self::Good),
            "GREAT" => Some(Self::Great),
            _ => None,
        }
    }

    const fn is_negative(self) -> bool {
        matches!(self, Self::Awful | Self::Bad)
    }

    const fn is_positive(self) -> bool {
        matches!(self, Self::Good | Self::Great)
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum MutationSourceType {
    Activation,
    Effect,
    Bonus,
    None,
}

impl MutationSourceType {
    fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "MUT_TYPE_ACTIVATION" => Some(Self::Activation),
            "MUT_TYPE_EFFECT" => Some(Self::Effect),
            "MUT_TYPE_BONUS" => Some(Self::Bonus),
            "0" => Some(Self::None),
            _ => None,
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Activation => "activation",
            Self::Effect => "effect",
            Self::Bonus => "bonus",
            Self::None => "none",
        }
    }

    const fn mechanism_family(self) -> &'static str {
        match self {
            Self::Activation => "activation",
            Self::Effect => "periodic-effect",
            Self::Bonus => "passive-bonus",
            Self::None => "cross-system-query",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum MutationCoverageStatus {
    Active,
    MechanicsReady,
    Blocked,
}

impl MutationCoverageStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::MechanicsReady => "mechanics-ready",
            Self::Blocked => "blocked",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MutationPlan {
    schema_version: u16,
    source_commit: String,
    global_rules: Vec<String>,
    mutations: Vec<MutationPlanEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MutationPlanEntry {
    source_index: u16,
    source_constant: String,
    source_handler: String,
    id: String,
    name_zh: String,
    description_zh: String,
    rating: MutationRating,
    random_weight: u8,
    source_type: MutationSourceType,
    mechanism_family: String,
    #[serde(default)]
    eligibility_conditions: Vec<String>,
    #[serde(default)]
    weight_conditions: Vec<String>,
    #[serde(default)]
    removes_on_gain: Vec<String>,
    status: MutationCoverageStatus,
    #[serde(default)]
    blockers: Vec<String>,
    #[serde(default)]
    source_no_numeric_effect: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceMutation {
    source_index: u16,
    source_constant: String,
    source_handler: String,
    id: String,
    name_zh: String,
    description_zh: String,
    rating: MutationRating,
    random_weight: u8,
    source_type: MutationSourceType,
    mechanism_family: String,
    eligibility_conditions: Vec<String>,
    weight_conditions: Vec<String>,
    removes_on_gain: Vec<String>,
    source_no_numeric_effect: bool,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DemoMutationCoverageReport {
    pub schema_version: u16,
    pub source_commit: String,
    pub mutations_total: usize,
    pub base_random_candidates: usize,
    pub zero_weight_mutations: usize,
    pub source_type_counts: BTreeMap<String, usize>,
    pub random_candidate_type_counts: BTreeMap<String, usize>,
    pub mechanism_family_counts: BTreeMap<String, usize>,
    pub status_counts: BTreeMap<String, usize>,
    pub eligibility_condition_entries: usize,
    pub eligibility_conditions_total: usize,
    pub weight_conditions_total: usize,
    pub mutual_exclusion_edges: usize,
    pub source_no_numeric_effect_ids: Vec<String>,
}

fn invalid(message: impl Into<String>) -> LegacyImportError {
    LegacyImportError::InvalidDemoMutationAudit(message.into())
}

fn mutation_id(source_constant: &str) -> String {
    format!(
        "rfb.mutation.{}",
        source_constant
            .strip_prefix("MUT_")
            .unwrap_or(source_constant)
            .to_ascii_lowercase()
            .replace('_', "-")
    )
}

fn parse_constants(source: &str) -> Result<BTreeMap<u16, String>, LegacyImportError> {
    let mut constants = BTreeMap::new();
    for line in source.lines() {
        let mut fields = line.split_whitespace();
        if fields.next() != Some("#define") {
            continue;
        }
        let Some(name) = fields.next().filter(|name| name.starts_with("MUT_")) else {
            continue;
        };
        let Some(index) = fields.next().and_then(|value| value.parse::<u16>().ok()) else {
            continue;
        };
        if usize::from(index) < MUTATION_COUNT && constants.insert(index, name.to_owned()).is_some()
        {
            return Err(invalid(format!(
                "duplicate mutation constant index {index}"
            )));
        }
    }
    Ok(constants)
}

fn function_body<'a>(sources: &'a [String], function: &str) -> Option<&'a str> {
    let needle = format!("void {function}(");
    for source in sources {
        let Some(start) = source.find(&needle) else {
            continue;
        };
        let open = start + source[start..].find('{')?;
        let mut depth = 0_u32;
        for (offset, byte) in source.as_bytes()[open..].iter().enumerate() {
            match byte {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(&source[open..=open + offset]);
                    }
                }
                _ => {}
            }
        }
    }
    None
}

fn case_section<'a>(body: &'a str, case_name: &str) -> Option<&'a str> {
    let marker = format!("case {case_name}:");
    let marker_start = body.find(&marker)?;
    let case_depth = body[..marker_start].bytes().fold(0_i32, |depth, byte| {
        depth
            + match byte {
                b'{' => 1,
                b'}' => -1,
                _ => 0,
            }
    });
    let start = marker_start + marker.len();
    let remaining = &body[start..];
    let mut offset = 0;
    let mut depth = case_depth;
    for line in remaining.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if offset > 0
            && depth == case_depth
            && (trimmed.starts_with("case ") || trimmed.starts_with("default:"))
        {
            return Some(&remaining[..offset]);
        }
        depth = line.bytes().fold(depth, |depth, byte| {
            depth
                + match byte {
                    b'{' => 1,
                    b'}' => -1,
                    _ => 0,
                }
        });
        offset += line.len();
    }
    Some(remaining)
}

fn c_string(section: &str) -> Option<String> {
    let call = section.find("var_set_string")?;
    let source = &section[call..];
    let mut chars = source[source.find('"')? + 1..].chars();
    let mut result = String::new();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => return Some(result),
            '\\' => match chars.next()? {
                'n' => result.push('\n'),
                'r' => result.push('\r'),
                't' => result.push('\t'),
                '"' => result.push('"'),
                '\\' => result.push('\\'),
                escaped => {
                    result.push('\\');
                    result.push(escaped);
                }
            },
            value => result.push(value),
        }
    }
    None
}

fn case_string(body: &str, case_name: &str) -> Option<String> {
    c_string(case_section(body, case_name)?)
}

fn removed_mutations(body: &str) -> Vec<String> {
    let Some(section) = case_section(body, "SPELL_GAIN_MUT") else {
        return Vec::new();
    };
    section
        .split("mut_lose(")
        .skip(1)
        .filter_map(|tail| tail.split(')').next())
        .map(str::trim)
        .filter(|constant| constant.starts_with("MUT_"))
        .map(mutation_id)
        .collect()
}

fn eligibility_conditions(source_constant: &str, source_type: MutationSourceType) -> Vec<String> {
    let mut conditions = Vec::new();
    if source_type == MutationSourceType::Activation {
        conditions.push("exclude-class:berserker".to_owned());
    }
    if matches!(source_constant, "MUT_HP_TO_SP" | "MUT_SP_TO_HP") {
        conditions.push("exclude-race:water-elemental".to_owned());
    }
    match source_constant {
        "MUT_CHAOS_GIFT" => conditions.extend(
            [
                "exclude-class:chaos-warrior",
                "exclude-personality:chaotic",
                "exclude-mutation:purple-gift",
            ]
            .map(str::to_owned),
        ),
        "MUT_PURPLE_GIFT" => conditions.extend(
            ["exclude-class:chaos-warrior", "exclude-mutation:chaos-gift"].map(str::to_owned),
        ),
        "MUT_BAD_LUCK" => conditions.push("exclude-locked-mutation:good-luck".to_owned()),
        "MUT_GOOD_LUCK" => conditions.push("exclude-locked-mutation:bad-luck".to_owned()),
        "MUT_BEAK" | "MUT_TRUNK" | "MUT_XTRA_LEGS" => {
            conditions.push("exclude-race:igor".to_owned());
        }
        "MUT_BLINK" => {
            conditions.extend(["exclude-race:igor", "exclude-race:gnome"].map(str::to_owned))
        }
        "MUT_IMPOTENCE" => conditions.push("exclude-sex:female".to_owned()),
        "MUT_MIDAS_TOUCH" => conditions.push("exclude-class:alchemist".to_owned()),
        _ => {}
    }
    if matches!(
        source_constant,
        "MUT_SCORPION_TAIL"
            | "MUT_HORNS"
            | "MUT_BEAK"
            | "MUT_TRUNK"
            | "MUT_TENTACLES"
            | "MUT_FLESH_ROT"
            | "MUT_ALBINO"
            | "MUT_XTRA_LEGS"
            | "MUT_SHORT_LEG"
            | "MUT_SENSITIVE_EYES"
    ) {
        conditions.push("exclude-races:monster-ring,monster-sword,monster-armor".to_owned());
    }
    conditions
}

fn weight_conditions(
    source_constant: &str,
    rating: MutationRating,
    random_weight: u8,
) -> Vec<String> {
    let mut conditions = Vec::new();
    if random_weight > 0 && rating.is_negative() {
        conditions.push("gain:mutation-good-luck=>weight-1".to_owned());
    }
    if random_weight > 0 && rating.is_positive() {
        conditions.push("gain:mutation-bad-luck-or-personality-fragile=>weight-1".to_owned());
    }
    if rating.is_positive() {
        conditions.push("lose:mutation-good-luck=>weight-1".to_owned());
    }
    if rating.is_negative() {
        conditions.push("lose:mutation-bad-luck=>weight-1".to_owned());
    }
    match source_constant {
        "MUT_HYPN_GAZE" => conditions.push("gain:race-vampire=>weight-50".to_owned()),
        "MUT_HORNS" => conditions.push("gain:race-imp=>weight-50".to_owned()),
        "MUT_SHRIEK" => conditions.push("gain:race-yeek=>weight-50".to_owned()),
        "MUT_POLYMORPH" => conditions.push("gain:race-beastman=>weight-50".to_owned()),
        "MUT_TENTACLES" => conditions.push("gain:race-mind-flayer=>weight-50".to_owned()),
        _ => {}
    }
    conditions
}

fn parse_source_mutations(
    mut_h: &str,
    mut_c: &str,
    function_sources: &[String],
) -> Result<Vec<SourceMutation>, LegacyImportError> {
    let constants = parse_constants(mut_h)?;
    let rows = mut_c
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("{MUT_RATING_"))
        .collect::<Vec<_>>();
    if rows.len() != constants.len() {
        return Err(invalid(format!(
            "mutation constants/table mismatch: {} constants, {} rows",
            constants.len(),
            rows.len()
        )));
    }
    let mut mutations = Vec::with_capacity(rows.len());
    for (index, row) in rows.into_iter().enumerate() {
        let fields = row.splitn(5, ',').collect::<Vec<_>>();
        if fields.len() != 5 {
            return Err(invalid(format!(
                "invalid mutation table row {index}: {row}"
            )));
        }
        let source_index = u16::try_from(index).expect("mutation index must fit u16");
        let source_constant = constants
            .get(&source_index)
            .ok_or_else(|| invalid(format!("missing mutation constant for index {index}")))?
            .clone();
        let rating = MutationRating::parse(fields[0])
            .ok_or_else(|| invalid(format!("invalid mutation rating at index {index}")))?;
        let source_type = MutationSourceType::parse(fields[1])
            .ok_or_else(|| invalid(format!("invalid mutation type at index {index}")))?;
        let random_weight = fields[3]
            .trim()
            .parse::<u8>()
            .map_err(|_| invalid(format!("invalid mutation weight at index {index}")))?;
        let source_handler = fields[4]
            .trim()
            .trim_end_matches(',')
            .trim_end_matches('}')
            .rsplit(',')
            .next()
            .unwrap_or_default()
            .trim()
            .to_owned();
        let body = function_body(function_sources, &source_handler).ok_or_else(|| {
            invalid(format!(
                "missing source handler {source_handler} for index {index}"
            ))
        })?;
        let name_body = if source_handler == "draconian_kin_mut" {
            function_body(function_sources, "summon_kin_spell")
                .ok_or_else(|| invalid("missing delegated summon_kin_spell"))?
        } else {
            body
        };
        let name_zh = case_string(name_body, "SPELL_NAME").ok_or_else(|| {
            invalid(format!(
                "handler {source_handler} has no literal authoritative name"
            ))
        })?;
        let description_zh = case_string(body, "SPELL_MUT_DESC").ok_or_else(|| {
            invalid(format!(
                "handler {source_handler} has no literal mutation description"
            ))
        })?;
        mutations.push(SourceMutation {
            source_index,
            id: mutation_id(&source_constant),
            eligibility_conditions: eligibility_conditions(&source_constant, source_type),
            weight_conditions: weight_conditions(&source_constant, rating, random_weight),
            removes_on_gain: removed_mutations(body),
            source_no_numeric_effect: source_constant == "MUT_MERCHANTS_FRIEND",
            mechanism_family: source_type.mechanism_family().to_owned(),
            source_constant,
            source_handler,
            name_zh,
            description_zh,
            rating,
            random_weight,
            source_type,
        });
    }
    Ok(mutations)
}

fn validate_entry(
    entry: &MutationPlanEntry,
    source: &SourceMutation,
) -> Result<(), LegacyImportError> {
    let immutable_matches = entry.source_index == source.source_index
        && entry.source_constant == source.source_constant
        && entry.source_handler == source.source_handler
        && entry.id == source.id
        && entry.name_zh == source.name_zh
        && entry.description_zh == source.description_zh
        && entry.rating == source.rating
        && entry.random_weight == source.random_weight
        && entry.source_type == source.source_type
        && entry.mechanism_family == source.mechanism_family
        && entry.eligibility_conditions == source.eligibility_conditions
        && entry.weight_conditions == source.weight_conditions
        && entry.removes_on_gain == source.removes_on_gain
        && entry.source_no_numeric_effect == source.source_no_numeric_effect;
    if !immutable_matches {
        return Err(invalid(format!(
            "mutation source index {} does not match authoritative master metadata",
            entry.source_index
        )));
    }
    let unique_blockers = entry.blockers.iter().collect::<BTreeSet<_>>();
    if unique_blockers.len() != entry.blockers.len()
        || entry
            .blockers
            .iter()
            .any(|blocker| blocker.trim().is_empty())
        || (entry.status == MutationCoverageStatus::Blocked) == entry.blockers.is_empty()
    {
        return Err(invalid(format!(
            "mutation {} has invalid status/blockers",
            entry.id
        )));
    }
    Ok(())
}

fn build_report(
    source_commit: &str,
    source_mutations: &[SourceMutation],
    plan: &MutationPlan,
) -> Result<DemoMutationCoverageReport, LegacyImportError> {
    if plan.schema_version != 1
        || plan.source_commit != source_commit
        || plan.global_rules != GLOBAL_RULES.map(str::to_owned)
        || source_mutations.len() != MUTATION_COUNT
        || plan.mutations.len() != MUTATION_COUNT
    {
        return Err(invalid(
            "mutation plan schema, source commit, global rules, or total count is stale",
        ));
    }
    let mut ids = BTreeSet::new();
    for (entry, source) in plan.mutations.iter().zip(source_mutations) {
        if !ids.insert(entry.id.as_str()) {
            return Err(invalid(format!("duplicate mutation id {}", entry.id)));
        }
        validate_entry(entry, source)?;
    }

    let mut source_type_counts = BTreeMap::new();
    let mut random_candidate_type_counts = BTreeMap::new();
    let mut mechanism_family_counts = BTreeMap::new();
    let mut status_counts = BTreeMap::new();
    for (source, entry) in source_mutations.iter().zip(&plan.mutations) {
        *source_type_counts
            .entry(source.source_type.as_str().to_owned())
            .or_default() += 1;
        if source.random_weight > 0 {
            *random_candidate_type_counts
                .entry(source.source_type.as_str().to_owned())
                .or_default() += 1;
        }
        *mechanism_family_counts
            .entry(source.mechanism_family.clone())
            .or_default() += 1;
        *status_counts
            .entry(entry.status.as_str().to_owned())
            .or_default() += 1;
    }
    let base_random_candidates = source_mutations
        .iter()
        .filter(|mutation| mutation.random_weight > 0)
        .count();
    Ok(DemoMutationCoverageReport {
        schema_version: 1,
        source_commit: source_commit.to_owned(),
        mutations_total: source_mutations.len(),
        base_random_candidates,
        zero_weight_mutations: source_mutations.len() - base_random_candidates,
        source_type_counts,
        random_candidate_type_counts,
        mechanism_family_counts,
        status_counts,
        eligibility_condition_entries: source_mutations
            .iter()
            .filter(|mutation| !mutation.eligibility_conditions.is_empty())
            .count(),
        eligibility_conditions_total: source_mutations
            .iter()
            .map(|mutation| mutation.eligibility_conditions.len())
            .sum(),
        weight_conditions_total: source_mutations
            .iter()
            .map(|mutation| mutation.weight_conditions.len())
            .sum(),
        mutual_exclusion_edges: source_mutations
            .iter()
            .map(|mutation| mutation.removes_on_gain.len())
            .sum(),
        source_no_numeric_effect_ids: source_mutations
            .iter()
            .filter(|mutation| mutation.source_no_numeric_effect)
            .map(|mutation| mutation.id.clone())
            .collect(),
    })
}

pub fn audit_demo_mutations(
    source: &Path,
    plan_path: &Path,
) -> Result<DemoMutationCoverageReport, LegacyImportError> {
    let source_commit = resolve_legacy_content_commit(source)?;
    let mut_h = read_legacy_object_at(source, &source_commit, "src/mut.h")?;
    let mut_c = read_legacy_object_at(source, &source_commit, "src/mut.c")?;
    let function_sources = MUTATION_SOURCE_PATHS
        .iter()
        .map(|path| read_legacy_object_at(source, &source_commit, path))
        .collect::<Result<Vec<_>, _>>()?;
    let source_mutations = parse_source_mutations(&mut_h, &mut_c, &function_sources)?;
    let plan: MutationPlan = serde_json::from_slice(&fs::read(plan_path)?)?;
    build_report(&source_commit, &source_mutations, &plan)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_parser_extracts_identity_rules_and_gain_conflicts() {
        let mut_h = "#define MUT_SPIT_ACID 0\n#define MUT_NORMALITY 1\n";
        let mut_c = r#"
            {MUT_RATING_GOOD, MUT_TYPE_ACTIVATION, A_DEX, 8, {9, 9, 30, spit_acid_spell}},
            {MUT_RATING_BAD, MUT_TYPE_EFFECT, 0, 2, {0, 0, 0, normality_mut}},
        "#;
        let functions = vec![
            r#"
            void spit_acid_spell(int cmd, variant *res) {
                switch (cmd) {
                case SPELL_NAME: var_set_string(res, "喷吐强酸"); break;
                case SPELL_GAIN_MUT: mut_lose(MUT_NORMALITY); break;
                case SPELL_MUT_DESC: var_set_string(res, "你能够喷吐强酸。"); break;
                }
            }
            void normality_mut(int cmd, variant *res) {
                switch (cmd) {
                case SPELL_NAME: var_set_string(res, "常态"); break;
                case SPELL_MUT_DESC: var_set_string(res, "你会恢复正常。"); break;
                case SPELL_PROCESS: break;
                }
            }
        "#
            .to_owned(),
        ];

        let parsed = parse_source_mutations(mut_h, mut_c, &functions).expect("valid source");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].id, "rfb.mutation.spit-acid");
        assert_eq!(parsed[0].name_zh, "喷吐强酸");
        assert_eq!(parsed[0].description_zh, "你能够喷吐强酸。");
        assert_eq!(
            parsed[0].eligibility_conditions,
            ["exclude-class:berserker"]
        );
        assert_eq!(parsed[0].removes_on_gain, ["rfb.mutation.normality"]);
        assert_eq!(parsed[1].source_type, MutationSourceType::Effect);
    }
}
