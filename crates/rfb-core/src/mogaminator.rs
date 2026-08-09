// SPDX-License-Identifier: MPL-2.0

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MogaminatorProgram {
    pub lines: Vec<MogaminatorLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledMogaminator {
    pub program: MogaminatorProgram,
    pub rules: Vec<CompiledMogaminatorRule>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledMogaminatorRule {
    pub line_number: usize,
    pub condition: Option<MogaminatorExpression>,
    pub rule: MogaminatorRule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MogaminatorLine {
    pub line_number: usize,
    pub source: String,
    pub kind: MogaminatorLineKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MogaminatorLineKind {
    Blank,
    Comment,
    /// Replaces the one active condition for every following rule, matching
    /// the original preference-file behavior.
    Condition(MogaminatorExpression),
    Rule(MogaminatorRule),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MogaminatorRule {
    pub action: MogaminatorAction,
    pub predicates: Vec<MogaminatorPredicate>,
    pub search: MogaminatorSearch,
    pub inscription: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MogaminatorAction {
    pub disposition: MogaminatorDisposition,
    pub display: bool,
    pub auto_identify: bool,
}

impl Default for MogaminatorAction {
    fn default() -> Self {
        Self {
            disposition: MogaminatorDisposition::PickUp,
            display: true,
            auto_identify: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MogaminatorDisposition {
    PickUp,
    Destroy,
    Leave,
    Query,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MogaminatorSearch {
    pub text: String,
    pub anchored_at_start: bool,
    pub anchored_at_end: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MogaminatorPredicate {
    All,
    Unsensed,
    Unidentified,
    Identified,
    FullyIdentified,
    Unaware,
    Average,
    Good,
    Cursed,
    Ego,
    Artifact,
    Nameless,
    Rare,
    Common,
    Worthless,
    DiceBoosted,
    Icky,
    Unreadable,
    FirstRealm,
    SecondRealm,
    FirstBook,
    SecondBook,
    ThirdBook,
    FourthBook,
    Collecting,
    Special,
    Unusable,
    Wanted,
    Unique,
    Human,
    MoreDiceThan(u32),
    MoreBonusThan(u32),
    MoreLevelThan(u32),
    MoreWeightThan(u32),
    MoreChargesThan(u32),
    MoreValueThan(u32),
    Items,
    Weapons,
    FavoriteWeapons,
    HaftedWeapons,
    Diggers,
    Shooters,
    Ammo,
    Arrows,
    Armors,
    Shields,
    Suits,
    Cloaks,
    Helms,
    Gloves,
    Boots,
    Lights,
    Rings,
    Amulets,
    Spellbooks,
    Wands,
    Staves,
    Rods,
    Potions,
    Scrolls,
    Junk,
    Corpses,
    Skeletons,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MogaminatorExpression {
    Call {
        function: MogaminatorFunction,
        arguments: Vec<MogaminatorExpression>,
    },
    Variable(MogaminatorVariable),
    Literal(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MogaminatorFunction {
    Or,
    And,
    Not,
    Equal,
    LessOrEqual,
    GreaterOrEqual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MogaminatorVariable {
    System,
    Keyboard,
    Graphics,
    Monochrome,
    Race,
    Class,
    Subclass,
    Speciality,
    Player,
    FirstRealm,
    SecondRealm,
    Level,
    AutoRegister,
    Money,
    Selling,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MogaminatorDiagnostic {
    pub line: usize,
    pub column: usize,
    pub code: MogaminatorDiagnosticCode,
    pub arguments: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MogaminatorDiagnosticCode {
    EmptyCondition,
    ExpectedConditionFunction,
    UnknownConditionFunction,
    UnknownConditionVariable,
    UnterminatedConditionExpression,
    UnexpectedConditionClose,
    TrailingConditionInput,
    InvalidNumericPredicate,
    MissingRuleBody,
    UnsupportedPredicate,
    UnsupportedVariable,
    InvalidConditionArity,
}

impl MogaminatorDiagnosticCode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EmptyCondition => "mogaminator.empty-condition",
            Self::ExpectedConditionFunction => "mogaminator.expected-condition-function",
            Self::UnknownConditionFunction => "mogaminator.unknown-condition-function",
            Self::UnknownConditionVariable => "mogaminator.unknown-condition-variable",
            Self::UnterminatedConditionExpression => {
                "mogaminator.unterminated-condition-expression"
            }
            Self::UnexpectedConditionClose => "mogaminator.unexpected-condition-close",
            Self::TrailingConditionInput => "mogaminator.trailing-condition-input",
            Self::InvalidNumericPredicate => "mogaminator.invalid-numeric-predicate",
            Self::MissingRuleBody => "mogaminator.missing-rule-body",
            Self::UnsupportedPredicate => "mogaminator.unsupported-predicate",
            Self::UnsupportedVariable => "mogaminator.unsupported-variable",
            Self::InvalidConditionArity => "mogaminator.invalid-condition-arity",
        }
    }
}

const ADJECTIVES: &[(&str, MogaminatorPredicate)] = &[
    ("*identified*", MogaminatorPredicate::FullyIdentified),
    ("*已鉴定*", MogaminatorPredicate::FullyIdentified),
    ("dice boosted", MogaminatorPredicate::DiceBoosted),
    ("伤害骰提升", MogaminatorPredicate::DiceBoosted),
    ("first realm's", MogaminatorPredicate::FirstRealm),
    ("第一领域的", MogaminatorPredicate::FirstRealm),
    ("second realm's", MogaminatorPredicate::SecondRealm),
    ("第二领域的", MogaminatorPredicate::SecondRealm),
    ("unidentified", MogaminatorPredicate::Unidentified),
    ("未鉴定", MogaminatorPredicate::Unidentified),
    ("identified", MogaminatorPredicate::Identified),
    ("已鉴定", MogaminatorPredicate::Identified),
    ("collecting", MogaminatorPredicate::Collecting),
    ("收集中", MogaminatorPredicate::Collecting),
    ("unreadable", MogaminatorPredicate::Unreadable),
    ("无法阅读", MogaminatorPredicate::Unreadable),
    ("unsensed", MogaminatorPredicate::Unsensed),
    ("未感知", MogaminatorPredicate::Unsensed),
    ("unaware", MogaminatorPredicate::Unaware),
    ("未察觉", MogaminatorPredicate::Unaware),
    ("average", MogaminatorPredicate::Average),
    ("普通的", MogaminatorPredicate::Average),
    ("good", MogaminatorPredicate::Good),
    ("良好的", MogaminatorPredicate::Good),
    ("cursed", MogaminatorPredicate::Cursed),
    ("被诅咒的", MogaminatorPredicate::Cursed),
    ("artifact", MogaminatorPredicate::Artifact),
    ("神器", MogaminatorPredicate::Artifact),
    ("nameless", MogaminatorPredicate::Nameless),
    ("无名", MogaminatorPredicate::Nameless),
    ("worthless", MogaminatorPredicate::Worthless),
    ("无价值", MogaminatorPredicate::Worthless),
    ("special", MogaminatorPredicate::Special),
    ("特殊的", MogaminatorPredicate::Special),
    ("unusable", MogaminatorPredicate::Unusable),
    ("不可用", MogaminatorPredicate::Unusable),
    ("wanted", MogaminatorPredicate::Wanted),
    ("悬赏中", MogaminatorPredicate::Wanted),
    ("unique", MogaminatorPredicate::Unique),
    ("唯一", MogaminatorPredicate::Unique),
    ("human", MogaminatorPredicate::Human),
    ("人类", MogaminatorPredicate::Human),
    ("first", MogaminatorPredicate::FirstBook),
    ("第一", MogaminatorPredicate::FirstBook),
    ("second", MogaminatorPredicate::SecondBook),
    ("第二", MogaminatorPredicate::SecondBook),
    ("third", MogaminatorPredicate::ThirdBook),
    ("第三", MogaminatorPredicate::ThirdBook),
    ("fourth", MogaminatorPredicate::FourthBook),
    ("第四", MogaminatorPredicate::FourthBook),
    ("rare", MogaminatorPredicate::Rare),
    ("稀有", MogaminatorPredicate::Rare),
    ("common", MogaminatorPredicate::Common),
    ("常见", MogaminatorPredicate::Common),
    ("icky", MogaminatorPredicate::Icky),
    ("被排斥", MogaminatorPredicate::Icky),
    ("ego", MogaminatorPredicate::Ego),
    ("词缀", MogaminatorPredicate::Ego),
    ("all", MogaminatorPredicate::All),
    ("全部", MogaminatorPredicate::All),
];

const NOUNS: &[(&str, MogaminatorPredicate)] = &[
    ("favorite weapons", MogaminatorPredicate::FavoriteWeapons),
    ("偏好的武器", MogaminatorPredicate::FavoriteWeapons),
    ("hafted weapons", MogaminatorPredicate::HaftedWeapons),
    ("钝器", MogaminatorPredicate::HaftedWeapons),
    ("spellbooks", MogaminatorPredicate::Spellbooks),
    ("法术书", MogaminatorPredicate::Spellbooks),
    ("skeletons", MogaminatorPredicate::Skeletons),
    ("残骸", MogaminatorPredicate::Skeletons),
    ("shooters", MogaminatorPredicate::Shooters),
    ("发射器", MogaminatorPredicate::Shooters),
    ("weapons", MogaminatorPredicate::Weapons),
    ("武器", MogaminatorPredicate::Weapons),
    ("diggers", MogaminatorPredicate::Diggers),
    ("挖掘工具", MogaminatorPredicate::Diggers),
    ("armors", MogaminatorPredicate::Armors),
    ("防具", MogaminatorPredicate::Armors),
    ("shields", MogaminatorPredicate::Shields),
    ("盾牌", MogaminatorPredicate::Shields),
    ("suits", MogaminatorPredicate::Suits),
    ("护甲", MogaminatorPredicate::Suits),
    ("cloaks", MogaminatorPredicate::Cloaks),
    ("斗篷", MogaminatorPredicate::Cloaks),
    ("helms", MogaminatorPredicate::Helms),
    ("头盔", MogaminatorPredicate::Helms),
    ("gloves", MogaminatorPredicate::Gloves),
    ("手套", MogaminatorPredicate::Gloves),
    ("boots", MogaminatorPredicate::Boots),
    ("靴子", MogaminatorPredicate::Boots),
    ("lights", MogaminatorPredicate::Lights),
    ("光源", MogaminatorPredicate::Lights),
    ("rings", MogaminatorPredicate::Rings),
    ("戒指", MogaminatorPredicate::Rings),
    ("amulets", MogaminatorPredicate::Amulets),
    ("护身符", MogaminatorPredicate::Amulets),
    ("potions", MogaminatorPredicate::Potions),
    ("药水", MogaminatorPredicate::Potions),
    ("scrolls", MogaminatorPredicate::Scrolls),
    ("卷轴", MogaminatorPredicate::Scrolls),
    ("corpses", MogaminatorPredicate::Corpses),
    ("尸体", MogaminatorPredicate::Corpses),
    ("items", MogaminatorPredicate::Items),
    ("物品", MogaminatorPredicate::Items),
    ("ammo", MogaminatorPredicate::Ammo),
    ("弹药", MogaminatorPredicate::Ammo),
    ("arrows", MogaminatorPredicate::Arrows),
    ("箭类", MogaminatorPredicate::Arrows),
    ("wands", MogaminatorPredicate::Wands),
    ("魔杖", MogaminatorPredicate::Wands),
    ("staves", MogaminatorPredicate::Staves),
    ("法杖", MogaminatorPredicate::Staves),
    ("rods", MogaminatorPredicate::Rods),
    ("魔棒", MogaminatorPredicate::Rods),
    ("junk", MogaminatorPredicate::Junk),
    ("垃圾", MogaminatorPredicate::Junk),
];

#[derive(Debug, Clone, Copy)]
enum NumericPredicate {
    Dice,
    Bonus,
    Level,
    Weight,
    Charges,
    Value,
}

const NUMERIC_PREDICATES: &[(&str, NumericPredicate, usize)] = &[
    ("more charges than", NumericPredicate::Charges, 3),
    ("充能多于", NumericPredicate::Charges, 3),
    ("more weight than", NumericPredicate::Weight, 2),
    ("重量多于", NumericPredicate::Weight, 2),
    ("more bonus than", NumericPredicate::Bonus, 2),
    ("加成多于", NumericPredicate::Bonus, 2),
    ("more level than", NumericPredicate::Level, 2),
    ("等级多于", NumericPredicate::Level, 2),
    ("more value than", NumericPredicate::Value, 6),
    ("价值多于", NumericPredicate::Value, 6),
    ("more dice than", NumericPredicate::Dice, 2),
    ("伤害骰多于", NumericPredicate::Dice, 2),
];

/// Parses a complete Mogaminator source document without evaluating rules.
///
/// All diagnostics are returned together so an editor can translate and show
/// every invalid line. A caller must reject the entire result when any
/// diagnostic is present.
pub fn parse_mogaminator(source: &str) -> Result<MogaminatorProgram, Vec<MogaminatorDiagnostic>> {
    let mut lines = Vec::new();
    let mut diagnostics = Vec::new();
    for (offset, source_line) in source.lines().enumerate() {
        let line_number = offset + 1;
        match parse_line(source_line, line_number) {
            Ok(kind) => lines.push(MogaminatorLine {
                line_number,
                source: source_line.to_owned(),
                kind,
            }),
            Err(diagnostic) => diagnostics.push(diagnostic),
        }
    }
    if diagnostics.is_empty() {
        Ok(MogaminatorProgram { lines })
    } else {
        Err(diagnostics)
    }
}

/// Compiles the M2-supported rule subset. Any unsupported semantic rejects
/// the complete document, including rules currently disabled by a condition.
pub fn compile_mogaminator(
    source: &str,
) -> Result<CompiledMogaminator, Vec<MogaminatorDiagnostic>> {
    let program = parse_mogaminator(source)?;
    let mut diagnostics = Vec::new();
    let mut condition = None;
    let mut rules = Vec::new();
    for line in &program.lines {
        match &line.kind {
            MogaminatorLineKind::Condition(expression) => {
                validate_expression(expression, line.line_number, &mut diagnostics);
                condition = Some(expression.clone());
            }
            MogaminatorLineKind::Rule(rule) => {
                for predicate in &rule.predicates {
                    if !predicate_is_supported(*predicate) {
                        diagnostics.push(MogaminatorDiagnostic {
                            line: line.line_number,
                            column: 1,
                            code: MogaminatorDiagnosticCode::UnsupportedPredicate,
                            arguments: vec![format!("{predicate:?}")],
                        });
                    }
                }
                rules.push(CompiledMogaminatorRule {
                    line_number: line.line_number,
                    condition: condition.clone(),
                    rule: rule.clone(),
                });
            }
            MogaminatorLineKind::Blank | MogaminatorLineKind::Comment => {}
        }
    }
    if diagnostics.is_empty() {
        Ok(CompiledMogaminator { program, rules })
    } else {
        Err(diagnostics)
    }
}

fn predicate_is_supported(predicate: MogaminatorPredicate) -> bool {
    matches!(
        predicate,
        MogaminatorPredicate::All
            | MogaminatorPredicate::Unsensed
            | MogaminatorPredicate::Unidentified
            | MogaminatorPredicate::Identified
            | MogaminatorPredicate::FullyIdentified
            | MogaminatorPredicate::Unaware
            | MogaminatorPredicate::Average
            | MogaminatorPredicate::Good
            | MogaminatorPredicate::Cursed
            | MogaminatorPredicate::Ego
            | MogaminatorPredicate::Artifact
            | MogaminatorPredicate::Nameless
            | MogaminatorPredicate::Rare
            | MogaminatorPredicate::Common
            | MogaminatorPredicate::Worthless
            | MogaminatorPredicate::DiceBoosted
            | MogaminatorPredicate::Icky
            | MogaminatorPredicate::Unreadable
            | MogaminatorPredicate::FirstRealm
            | MogaminatorPredicate::SecondRealm
            | MogaminatorPredicate::FirstBook
            | MogaminatorPredicate::SecondBook
            | MogaminatorPredicate::ThirdBook
            | MogaminatorPredicate::FourthBook
            | MogaminatorPredicate::Collecting
            | MogaminatorPredicate::Special
            | MogaminatorPredicate::Unusable
            | MogaminatorPredicate::Wanted
            | MogaminatorPredicate::Unique
            | MogaminatorPredicate::Human
            | MogaminatorPredicate::MoreDiceThan(_)
            | MogaminatorPredicate::MoreBonusThan(_)
            | MogaminatorPredicate::MoreLevelThan(_)
            | MogaminatorPredicate::MoreWeightThan(_)
            | MogaminatorPredicate::MoreChargesThan(_)
            | MogaminatorPredicate::MoreValueThan(_)
            | MogaminatorPredicate::Items
            | MogaminatorPredicate::Weapons
            | MogaminatorPredicate::FavoriteWeapons
            | MogaminatorPredicate::HaftedWeapons
            | MogaminatorPredicate::Diggers
            | MogaminatorPredicate::Shooters
            | MogaminatorPredicate::Ammo
            | MogaminatorPredicate::Arrows
            | MogaminatorPredicate::Armors
            | MogaminatorPredicate::Shields
            | MogaminatorPredicate::Suits
            | MogaminatorPredicate::Cloaks
            | MogaminatorPredicate::Helms
            | MogaminatorPredicate::Gloves
            | MogaminatorPredicate::Boots
            | MogaminatorPredicate::Lights
            | MogaminatorPredicate::Rings
            | MogaminatorPredicate::Amulets
            | MogaminatorPredicate::Spellbooks
            | MogaminatorPredicate::Wands
            | MogaminatorPredicate::Staves
            | MogaminatorPredicate::Rods
            | MogaminatorPredicate::Potions
            | MogaminatorPredicate::Scrolls
            | MogaminatorPredicate::Junk
            | MogaminatorPredicate::Corpses
            | MogaminatorPredicate::Skeletons
    )
}

fn validate_expression(
    expression: &MogaminatorExpression,
    line: usize,
    diagnostics: &mut Vec<MogaminatorDiagnostic>,
) {
    match expression {
        MogaminatorExpression::Call {
            function,
            arguments,
        } => {
            let valid_arity = match function {
                MogaminatorFunction::Or | MogaminatorFunction::And => !arguments.is_empty(),
                MogaminatorFunction::Not => !arguments.is_empty(),
                MogaminatorFunction::Equal
                | MogaminatorFunction::LessOrEqual
                | MogaminatorFunction::GreaterOrEqual => arguments.len() >= 2,
            };
            if !valid_arity {
                diagnostics.push(MogaminatorDiagnostic {
                    line,
                    column: 1,
                    code: MogaminatorDiagnosticCode::InvalidConditionArity,
                    arguments: vec![format!("{function:?}"), arguments.len().to_string()],
                });
            }
            for argument in arguments {
                validate_expression(argument, line, diagnostics);
            }
        }
        MogaminatorExpression::Variable(variable) => {
            if !matches!(
                variable,
                MogaminatorVariable::Race
                    | MogaminatorVariable::Class
                    | MogaminatorVariable::Subclass
                    | MogaminatorVariable::Speciality
                    | MogaminatorVariable::FirstRealm
                    | MogaminatorVariable::SecondRealm
                    | MogaminatorVariable::Level
                    | MogaminatorVariable::Money
                    | MogaminatorVariable::Selling
            ) {
                diagnostics.push(MogaminatorDiagnostic {
                    line,
                    column: 1,
                    code: MogaminatorDiagnosticCode::UnsupportedVariable,
                    arguments: vec![format!("{variable:?}")],
                });
            }
        }
        MogaminatorExpression::Literal(_) => {}
    }
}

#[must_use]
pub fn mogaminator_search_matches(search: &MogaminatorSearch, candidate: &str) -> bool {
    let candidate = candidate
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    if search.text.is_empty() {
        return true;
    }
    match (search.anchored_at_start, search.anchored_at_end) {
        (true, true) => candidate == search.text,
        (true, false) => candidate.starts_with(&search.text),
        (false, true) => candidate.ends_with(&search.text),
        (false, false) => candidate.contains(&search.text),
    }
}

fn parse_line(
    source: &str,
    line_number: usize,
) -> Result<MogaminatorLineKind, MogaminatorDiagnostic> {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Ok(MogaminatorLineKind::Blank);
    }
    if trimmed.starts_with('#') {
        return Ok(MogaminatorLineKind::Comment);
    }
    if let Some(condition) = trimmed.strip_prefix("?:") {
        return parse_condition(condition, source, line_number).map(MogaminatorLineKind::Condition);
    }
    parse_rule(trimmed, line_number).map(MogaminatorLineKind::Rule)
}

fn parse_condition(
    condition: &str,
    source_line: &str,
    line_number: usize,
) -> Result<MogaminatorExpression, MogaminatorDiagnostic> {
    let leading_columns = source_line
        .chars()
        .take_while(|char| char.is_whitespace())
        .count();
    let mut parser = ExpressionParser {
        source: condition,
        position: 0,
        line: line_number,
        base_column: leading_columns + 3,
    };
    parser.skip_whitespace();
    if parser.is_end() {
        return Err(parser.diagnostic(MogaminatorDiagnosticCode::EmptyCondition, Vec::new()));
    }
    let expression = parser.parse_expression()?;
    parser.skip_whitespace();
    if !parser.is_end() {
        return Err(parser.diagnostic(
            MogaminatorDiagnosticCode::TrailingConditionInput,
            vec![parser.remaining().to_owned()],
        ));
    }
    Ok(expression)
}

fn parse_rule(source: &str, line_number: usize) -> Result<MogaminatorRule, MogaminatorDiagnostic> {
    let mut action = MogaminatorAction::default();
    let mut body = source;
    while let Some(command) = body.chars().next() {
        let accepted = match command {
            '!' if action.disposition == MogaminatorDisposition::PickUp => {
                action.disposition = MogaminatorDisposition::Destroy;
                true
            }
            '~' if action.disposition == MogaminatorDisposition::PickUp => {
                action.disposition = MogaminatorDisposition::Leave;
                true
            }
            ';' if action.disposition == MogaminatorDisposition::PickUp => {
                action.disposition = MogaminatorDisposition::Query;
                true
            }
            '(' if action.display => {
                action.display = false;
                true
            }
            '?' => {
                action.auto_identify = true;
                true
            }
            _ => false,
        };
        if !accepted {
            break;
        }
        body = &body[command.len_utf8()..];
    }

    let (body, inscription) = body
        .split_once('#')
        .map_or((body, None), |(body, inscription)| {
            (body, Some(inscription.to_owned()))
        });
    let body = body.trim();
    if body.is_empty() {
        return Err(MogaminatorDiagnostic {
            line: line_number,
            column: source.chars().count() + 1,
            code: MogaminatorDiagnosticCode::MissingRuleBody,
            arguments: Vec::new(),
        });
    }

    let mut rest = body;
    let mut predicates = Vec::new();
    while let Some((predicate, remaining)) = match_keyword(rest, ADJECTIVES) {
        predicates.push(predicate);
        rest = remaining.trim_start();
    }

    let noun_start = rest;
    let predicate_count_before_noun = predicates.len();
    let mut has_noun = false;
    if let Some((noun, remaining)) = match_keyword(rest, NOUNS) {
        predicates.push(noun);
        rest = remaining.trim_start();
        has_noun = true;
    }

    if let Some((numeric, remaining)) = parse_numeric_predicate(rest, line_number)? {
        predicates.push(numeric);
        rest = remaining.trim_start();
    }

    let search_source = if let Some(search) = rest.strip_prefix(':') {
        search
    } else if rest.is_empty() {
        ""
    } else if predicates.len() > predicate_count_before_noun {
        predicates.truncate(predicate_count_before_noun);
        has_noun = false;
        noun_start
    } else {
        rest
    };
    if !has_noun && (search_source.is_empty() || rest.starts_with(':')) {
        predicates.push(MogaminatorPredicate::Items);
    }

    Ok(MogaminatorRule {
        action,
        predicates,
        search: normalize_search(search_source),
        inscription,
    })
}

fn match_keyword<'a>(
    source: &'a str,
    keywords: &[(&str, MogaminatorPredicate)],
) -> Option<(MogaminatorPredicate, &'a str)> {
    keywords.iter().find_map(|(keyword, predicate)| {
        strip_keyword(source, keyword).map(|remaining| (*predicate, remaining))
    })
}

fn strip_keyword<'a>(source: &'a str, keyword: &str) -> Option<&'a str> {
    let prefix = source.get(..keyword.len())?;
    if !prefix.eq_ignore_ascii_case(keyword) {
        return None;
    }
    let remaining = &source[keyword.len()..];
    remaining
        .chars()
        .next()
        .is_none_or(|next| next.is_whitespace() || next == ':')
        .then_some(remaining)
}

fn parse_numeric_predicate(
    source: &str,
    line: usize,
) -> Result<Option<(MogaminatorPredicate, &str)>, MogaminatorDiagnostic> {
    for (keyword, kind, max_digits) in NUMERIC_PREDICATES {
        let Some(remaining) = strip_keyword(source, keyword) else {
            continue;
        };
        let remaining = remaining.trim_start();
        let digits = remaining
            .char_indices()
            .take_while(|(_, char)| char.is_ascii_digit())
            .map(|(index, char)| index + char.len_utf8())
            .last()
            .unwrap_or(0);
        let value_text = &remaining[..digits];
        if value_text.is_empty() || value_text.len() > *max_digits {
            return Err(MogaminatorDiagnostic {
                line,
                column: source.chars().count() - remaining.chars().count() + 1,
                code: MogaminatorDiagnosticCode::InvalidNumericPredicate,
                arguments: vec![(*keyword).to_owned(), max_digits.to_string()],
            });
        }
        let value = value_text.parse::<u32>().expect("ASCII digits fit in u32");
        let predicate = match kind {
            NumericPredicate::Dice => MogaminatorPredicate::MoreDiceThan(value),
            NumericPredicate::Bonus => MogaminatorPredicate::MoreBonusThan(value),
            NumericPredicate::Level => MogaminatorPredicate::MoreLevelThan(value),
            NumericPredicate::Weight => MogaminatorPredicate::MoreWeightThan(value),
            NumericPredicate::Charges => MogaminatorPredicate::MoreChargesThan(value),
            NumericPredicate::Value => MogaminatorPredicate::MoreValueThan(value),
        };
        return Ok(Some((predicate, &remaining[digits..])));
    }
    Ok(None)
}

fn normalize_search(source: &str) -> MogaminatorSearch {
    let source = source.trim();
    let (anchored_at_start, source) = source
        .strip_prefix('^')
        .map_or((false, source), |source| (true, source));
    let (anchored_at_end, source) = source
        .strip_suffix('$')
        .map_or((false, source), |source| (true, source));
    MogaminatorSearch {
        text: source
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase(),
        anchored_at_start,
        anchored_at_end,
    }
}

struct ExpressionParser<'a> {
    source: &'a str,
    position: usize,
    line: usize,
    base_column: usize,
}

impl ExpressionParser<'_> {
    fn parse_expression(&mut self) -> Result<MogaminatorExpression, MogaminatorDiagnostic> {
        self.skip_whitespace();
        match self.peek() {
            Some('[') => self.parse_call(),
            Some(']') => Err(self.diagnostic(
                MogaminatorDiagnosticCode::UnexpectedConditionClose,
                Vec::new(),
            )),
            Some(_) => {
                let token = self.parse_token().to_owned();
                if token.starts_with('$') {
                    parse_variable(&token)
                        .map(MogaminatorExpression::Variable)
                        .ok_or_else(|| {
                            self.diagnostic(
                                MogaminatorDiagnosticCode::UnknownConditionVariable,
                                vec![token],
                            )
                        })
                } else {
                    Ok(MogaminatorExpression::Literal(token))
                }
            }
            None => Err(self.diagnostic(
                MogaminatorDiagnosticCode::UnterminatedConditionExpression,
                Vec::new(),
            )),
        }
    }

    fn parse_call(&mut self) -> Result<MogaminatorExpression, MogaminatorDiagnostic> {
        self.advance();
        self.skip_whitespace();
        if self.is_end() {
            return Err(self.diagnostic(
                MogaminatorDiagnosticCode::UnterminatedConditionExpression,
                Vec::new(),
            ));
        }
        if matches!(self.peek(), Some('[' | ']')) {
            return Err(self.diagnostic(
                MogaminatorDiagnosticCode::ExpectedConditionFunction,
                Vec::new(),
            ));
        }
        let token = self.parse_token().to_owned();
        let function = parse_function(&token).ok_or_else(|| {
            self.diagnostic(
                MogaminatorDiagnosticCode::UnknownConditionFunction,
                vec![token],
            )
        })?;
        let mut arguments = Vec::new();
        loop {
            self.skip_whitespace();
            match self.peek() {
                Some(']') => {
                    self.advance();
                    break;
                }
                Some(_) => arguments.push(self.parse_expression()?),
                None => {
                    return Err(self.diagnostic(
                        MogaminatorDiagnosticCode::UnterminatedConditionExpression,
                        Vec::new(),
                    ));
                }
            }
        }
        Ok(MogaminatorExpression::Call {
            function,
            arguments,
        })
    }

    fn parse_token(&mut self) -> &str {
        let start = self.position;
        while let Some(char) = self.peek() {
            if char.is_whitespace() || matches!(char, '[' | ']') {
                break;
            }
            self.advance();
        }
        &self.source[start..self.position]
    }

    fn skip_whitespace(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.advance();
        }
    }

    fn peek(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    fn advance(&mut self) {
        if let Some(char) = self.peek() {
            self.position += char.len_utf8();
        }
    }

    fn is_end(&self) -> bool {
        self.position == self.source.len()
    }

    fn remaining(&self) -> &str {
        &self.source[self.position..]
    }

    fn diagnostic(
        &self,
        code: MogaminatorDiagnosticCode,
        arguments: Vec<String>,
    ) -> MogaminatorDiagnostic {
        MogaminatorDiagnostic {
            line: self.line,
            column: self.base_column + self.source[..self.position].chars().count(),
            code,
            arguments,
        }
    }
}

fn parse_function(token: &str) -> Option<MogaminatorFunction> {
    match token {
        "IOR" | "OR" | "包含或" | "或" => Some(MogaminatorFunction::Or),
        "AND" | "与" => Some(MogaminatorFunction::And),
        "NOT" | "非" => Some(MogaminatorFunction::Not),
        "EQU" | "等于" => Some(MogaminatorFunction::Equal),
        "LEQ" | "小于等于" => Some(MogaminatorFunction::LessOrEqual),
        "GEQ" | "大于等于" => Some(MogaminatorFunction::GreaterOrEqual),
        _ => None,
    }
}

fn parse_variable(token: &str) -> Option<MogaminatorVariable> {
    match token {
        "$SYS" | "$系统" => Some(MogaminatorVariable::System),
        "$KEYBOARD" | "$键盘" => Some(MogaminatorVariable::Keyboard),
        "$GRAF" | "$图形" => Some(MogaminatorVariable::Graphics),
        "$MONOCHROME" | "$单色" => Some(MogaminatorVariable::Monochrome),
        "$RACE" | "$种族" => Some(MogaminatorVariable::Race),
        "$CLASS" | "$职业" => Some(MogaminatorVariable::Class),
        "$SUBCLASS" | "$子职业" => Some(MogaminatorVariable::Subclass),
        "$SPECIALITY" | "$专精" => Some(MogaminatorVariable::Speciality),
        "$PLAYER" | "$玩家" => Some(MogaminatorVariable::Player),
        "$REALM1" | "$第一领域" => Some(MogaminatorVariable::FirstRealm),
        "$REALM2" | "$第二领域" => Some(MogaminatorVariable::SecondRealm),
        "$LEVEL" | "$等级" => Some(MogaminatorVariable::Level),
        "$AUTOREGISTER" | "$自动注册" => Some(MogaminatorVariable::AutoRegister),
        "$MONEY" | "$金币" => Some(MogaminatorVariable::Money),
        "$SELLING" | "$交易" => Some(MogaminatorVariable::Selling),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_original_actions_bilingual_predicates_and_inscription() {
        let program = parse_mogaminator(
            "!?(未鉴定 良好的 武器 价值多于 500:^回声  刃$#@w1\n~unidentified good weapons more value than 500:^Echo   Blade$",
        )
        .expect("rules should parse");

        let MogaminatorLineKind::Rule(chinese) = &program.lines[0].kind else {
            panic!("expected Chinese rule");
        };
        assert_eq!(chinese.action.disposition, MogaminatorDisposition::Destroy);
        assert!(!chinese.action.display);
        assert!(chinese.action.auto_identify);
        assert_eq!(
            chinese.predicates,
            [
                MogaminatorPredicate::Unidentified,
                MogaminatorPredicate::Good,
                MogaminatorPredicate::Weapons,
                MogaminatorPredicate::MoreValueThan(500),
            ]
        );
        assert_eq!(
            chinese.search,
            MogaminatorSearch {
                text: "回声 刃".to_owned(),
                anchored_at_start: true,
                anchored_at_end: true,
            }
        );
        assert_eq!(chinese.inscription.as_deref(), Some("@w1"));

        let MogaminatorLineKind::Rule(english) = &program.lines[1].kind else {
            panic!("expected English rule");
        };
        assert_eq!(english.action.disposition, MogaminatorDisposition::Leave);
        assert_eq!(english.predicates, chinese.predicates);
        assert_eq!(english.search.text, "echo blade");
    }

    #[test]
    fn parses_conditions_with_english_and_chinese_aliases() {
        let program = parse_mogaminator(
            "?:[AND [EQU $CLASS Warrior] [GEQ $LEVEL 20]]\n?:[与 [等于 $职业 战士] [大于等于 $等级 20]]\n;items",
        )
        .expect("conditions should parse");

        for line in &program.lines[..2] {
            let MogaminatorLineKind::Condition(MogaminatorExpression::Call {
                function,
                arguments,
            }) = &line.kind
            else {
                panic!("expected condition call");
            };
            assert_eq!(*function, MogaminatorFunction::And);
            assert_eq!(arguments.len(), 2);
        }
        assert!(matches!(
            program.lines[2].kind,
            MogaminatorLineKind::Rule(MogaminatorRule {
                action: MogaminatorAction {
                    disposition: MogaminatorDisposition::Query,
                    ..
                },
                ..
            })
        ));
    }

    #[test]
    fn keeps_blank_comments_and_plain_text_searches() {
        let program = parse_mogaminator("\n  # comment\nStrange    Sword\n^回声  刃$")
            .expect("document should parse");
        assert_eq!(program.lines[0].kind, MogaminatorLineKind::Blank);
        assert_eq!(program.lines[1].kind, MogaminatorLineKind::Comment);
        let MogaminatorLineKind::Rule(rule) = &program.lines[2].kind else {
            panic!("expected plain search rule");
        };
        assert!(rule.predicates.is_empty());
        assert_eq!(rule.search.text, "strange sword");
        assert_eq!(program.lines[2].source, "Strange    Sword");
    }

    #[test]
    fn returns_stable_diagnostics_for_structural_errors() {
        let diagnostics = parse_mogaminator(
            "?:[UNKNOWN $LEVEL]\n?:[AND [EQU $LEVEL 20]\nweapons more value than many",
        )
        .expect_err("document should fail atomically");
        assert_eq!(diagnostics.len(), 3);
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code.as_str())
                .collect::<Vec<_>>(),
            [
                "mogaminator.unknown-condition-function",
                "mogaminator.unterminated-condition-expression",
                "mogaminator.invalid-numeric-predicate",
            ]
        );
        assert_eq!(diagnostics[0].arguments, ["UNKNOWN"]);
        assert_eq!(diagnostics[2].arguments, ["more value than", "6"]);
    }

    #[test]
    fn accepts_every_predicate_alias_and_action_symbol() {
        for (keyword, expected) in ADJECTIVES {
            let source = format!("{keyword}:needle");
            let program = parse_mogaminator(&source).expect("adjective alias should parse");
            let MogaminatorLineKind::Rule(rule) = &program.lines[0].kind else {
                panic!("expected adjective rule");
            };
            assert_eq!(rule.predicates.first(), Some(expected), "{keyword}");
        }
        for (keyword, expected) in NOUNS {
            let source = format!("{keyword}:needle");
            let program = parse_mogaminator(&source).expect("noun alias should parse");
            let MogaminatorLineKind::Rule(rule) = &program.lines[0].kind else {
                panic!("expected noun rule");
            };
            assert_eq!(rule.predicates, [*expected], "{keyword}");
        }
        for (prefix, disposition) in [
            ("", MogaminatorDisposition::PickUp),
            ("!", MogaminatorDisposition::Destroy),
            ("~", MogaminatorDisposition::Leave),
            (";", MogaminatorDisposition::Query),
        ] {
            let source = format!("{prefix}items");
            let program = parse_mogaminator(&source).expect("action should parse");
            let MogaminatorLineKind::Rule(rule) = &program.lines[0].kind else {
                panic!("expected action rule");
            };
            assert_eq!(rule.action.disposition, disposition, "{prefix}");
        }
    }

    #[test]
    fn compiler_rejects_unsupported_variables_even_below_false_condition() {
        let diagnostics = compile_mogaminator("?:[EQU 0 1]\n稀有 武器\n?:[EQU $SYS windows]\n物品")
            .expect_err("all active and inactive lines must compile atomically");
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code.as_str())
                .collect::<Vec<_>>(),
            ["mogaminator.unsupported-variable"]
        );

        compile_mogaminator("?:[与 [等于 $职业 战士] [大于等于 $等级 1]]\n良好的 武器")
            .expect("the M2 subset should compile");
    }
}
