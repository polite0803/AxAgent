//! 概念 / 行业 / 产业链 主题索引 + 本体对齐层
//!
//! 这是「#1 选股主题维度升级」的核心数据基础。它解决两件事：
//!
//! 1. **主题索引**：把知识图谱（或 vendor 数据）里的
//!    `股票 → 概念/行业` 关系倒排成 `概念 id → 成员股票集合`，
//!    让选股器能以「主题」为第一层候选宇宙，再叠加量化指标。
//! 2. **本体对齐（Ontology Alignment）**：用户查询词（如 "AI"）、各 vendor 命名
//!    （同花顺 "人工智能" / 东方财富 "AI概念" / 问财 "人工智能"）统一映射到
//!    **规范概念 id**（如 `concept_ai`）。否则开源知识库的概念命名和你们
//!    `ths/eastmoney/iwencai` 的命名对不上，就接不进 `screener` / `recommender`。
//!
//! 设计原则：
//! - 纯内存结构，**不依赖网络**，便于单测与在 `screen_snapshots` 中注入；
//! - 生产环境的成员数据应由 vendor（`astock-data`）填充，知识图谱仅作种子/补全；
//! - `ConceptIndex` 与 `harness` 解耦，落在 `stock-analysis`（implementor）内，
//!   不污染 `agent`（consumer）。

use std::collections::{HashMap, HashSet};

/// 概念节点的规范描述（本体注册项）
#[derive(Debug, Clone)]
pub struct ConceptNode {
    /// 规范 id，如 `concept_ai`
    pub id: String,
    /// 中文显示名，如 `人工智能`
    pub display: String,
    /// 别名集合（查询词 + 各 vendor 命名），用于本体对齐
    pub aliases: Vec<String>,
    /// 节点类型：`concept` | `industry` | `industry_chain`
    pub node_type: String,
}

impl ConceptNode {
    pub fn new(id: &str, display: &str, node_type: &str) -> Self {
        Self {
            id: id.to_string(),
            display: display.to_string(),
            aliases: Vec::new(),
            node_type: node_type.to_string(),
        }
    }

    /// 注册别名（查询词 / vendor 命名）
    pub fn with_aliases(mut self, aliases: &[&str]) -> Self {
        self.aliases = aliases.iter().map(|s| s.to_string()).collect();
        self
    }
}

/// 概念 / 行业主题索引（倒排：概念 id → 成员股票代码）
#[derive(Debug, Clone, Default)]
pub struct ConceptIndex {
    /// 规范 id → 节点元数据
    nodes: HashMap<String, ConceptNode>,
    /// 规范 id → 成员股票代码集合
    membership: HashMap<String, HashSet<String>>,
    /// 别名（小写归一化）→ 规范 id（本体对齐核心）
    alias_map: HashMap<String, String>,
}

impl ConceptIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册概念节点（含本体别名）
    pub fn register(&mut self, node: ConceptNode) {
        for alias in &node.aliases {
            self.alias_map.insert(normalize(alias), node.id.clone());
        }
        self.alias_map.insert(normalize(&node.display), node.id.clone());
        self.alias_map.insert(normalize(&node.id), node.id.clone());
        self.nodes.insert(node.id.clone(), node);
    }

    /// 添加「概念/行业 id → 成员股票」关系
    pub fn add_membership(&mut self, concept_id: &str, stock_code: &str) {
        self.membership.entry(concept_id.to_string()).or_default().insert(stock_code.to_string());
    }

    /// 从知识图谱边构建。
    ///
    /// 边格式 `(source, target, type)`：
    /// - `has_concept` / `in_industry`：表示 `source(股票) → target(概念/行业)`。
    ///   索引需要的是 `概念 → 股票`，故此处**反转方向**记录。
    /// - 其他边（如 `subsidiary_of` / `peer_of`）与主题成员无关，忽略。
    pub fn from_graph_edges(edges: &[(String, String, String)]) -> Self {
        let mut idx = Self::new();
        for (src, tgt, etype) in edges {
            if matches!(etype.as_str(), "has_concept" | "in_industry") {
                idx.add_membership(tgt, src);
            }
        }
        idx
    }

    /// 从边 CSV（`source,target,type` 三列，含表头）构建
    pub fn from_edge_csv(csv_text: &str) -> Self {
        let mut edges = Vec::new();
        for (i, line) in csv_text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || (i == 0 && line.starts_with("source,")) {
                continue; // 跳过表头
            }
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() == 3 {
                edges.push((parts[0].to_string(), parts[1].to_string(), parts[2].to_string()));
            }
        }
        Self::from_graph_edges(&edges)
    }

    /// 本体对齐：将查询词（用户别名 / vendor 命名）解析为规范概念 id
    pub fn resolve(&self, query: &str) -> Option<&str> {
        self.alias_map.get(&normalize(query)).map(|s| s.as_str())
    }

    /// 取概念成员股票集合
    pub fn members(&self, concept_id: &str) -> HashSet<String> {
        self.membership.get(concept_id).cloned().unwrap_or_default()
    }

    /// 显示名
    pub fn display(&self, concept_id: &str) -> Option<&str> {
        self.nodes.get(concept_id).map(|n| n.display.as_str())
    }

    /// 解析一组查询词为规范 id 列表（无法解析的忽略）
    pub fn resolve_many(&self, queries: &[String]) -> Vec<String> {
        queries.iter().filter_map(|q| self.resolve(q)).map(|s| s.to_string()).collect()
    }

    /// 计算主题候选宇宙：解析所有主题查询 → 取成员并集（mode=OR）或交集（mode=AND）
    pub fn theme_universe(&self, queries: &[String], require_all: bool) -> HashSet<String> {
        let ids = self.resolve_many(queries);
        if ids.is_empty() {
            return HashSet::new();
        }
        let sets: Vec<HashSet<String>> = ids.iter().map(|id| self.members(id)).collect();
        if require_all {
            let mut acc = sets[0].clone();
            for s in &sets[1..] {
                acc.retain(|c| s.contains(c));
            }
            acc
        } else {
            let mut acc = HashSet::new();
            for s in &sets {
                acc.extend(s.iter().cloned());
            }
            acc
        }
    }
}

fn normalize(s: &str) -> String {
    s.trim().to_lowercase()
}

/// 样例知识库的本体注册（演示「开源概念名 ↔ vendor 命名」对齐）。
///
/// 真实接入时由 `shinezai/QASystemOnFinancialKG` 或 vendor 数据自动填充，
/// 这里用 `edges.csv` 里出现的 5 个概念/行业节点作种子。
pub fn seed_sample_ontology(idx: &mut ConceptIndex) {
    idx.register(ConceptNode::new("concept_ai", "人工智能", "concept").with_aliases(&[
        "AI",
        "ai概念",
        "人工智能",
        "问财AI",
        "AI概念",
    ]));
    idx.register(ConceptNode::new("concept_chip", "芯片", "concept").with_aliases(&[
        "芯片",
        "半导体概念",
        "chip",
        "ic",
    ]));
    idx.register(ConceptNode::new("industry_semiconductor", "半导体", "industry").with_aliases(&[
        "半导体",
        "集成电路",
        "semiconductor",
    ]));
    idx.register(ConceptNode::new("industry_bank", "银行", "industry").with_aliases(&[
        "银行",
        "银行业",
        "bank",
    ]));
    idx.register(ConceptNode::new("industry_insurance", "保险", "industry").with_aliases(&[
        "保险",
        "保险业",
        "insurance",
    ]));
}

// ── A 股行业/概念本体种子（自动生成，来自 lemonhu/stock-knowledge-graph） ──
/// 49 行业, 163 概念（同花顺分类）
pub fn seed_ashare_ontology(idx: &mut ConceptIndex) {
    // === 行业 (49) ===
    idx.register(
        ConceptNode::new("f55b5c485b383665e660eefb03f589e0", "交通运输", "industry")
            .with_aliases(&["交通运输"]),
    );
    idx.register(
        ConceptNode::new("2212c3e783d2b83a8532d98eeba1c4b2", "仪器仪表", "industry")
            .with_aliases(&["仪器仪表"]),
    );
    idx.register(
        ConceptNode::new("891bb94e6b9ba56fa8229b1eac5214ad", "传媒娱乐", "industry")
            .with_aliases(&["传媒娱乐"]),
    );
    idx.register(
        ConceptNode::new("7dfd8d1571c918e44283bba8145076b3", "供水供气", "industry")
            .with_aliases(&["供水供气"]),
    );
    idx.register(
        ConceptNode::new("2aebe0c5cae88686f8861e2d4879d15a", "公路桥梁", "industry")
            .with_aliases(&["公路桥梁"]),
    );
    idx.register(
        ConceptNode::new("e8314e968735d8672f71b668d2db28f1", "其它行业", "industry")
            .with_aliases(&["其它行业"]),
    );
    idx.register(
        ConceptNode::new("e8d468e2bcec88651c422354c2f0d841", "农林牧渔", "industry")
            .with_aliases(&["农林牧渔"]),
    );
    idx.register(
        ConceptNode::new("8cf5cd1881ebc3c55ae649faa9247350", "农药化肥", "industry")
            .with_aliases(&["农药化肥"]),
    );
    idx.register(
        ConceptNode::new("77cbbbfcddcbeb7fc04ed2fe3f9e6d98", "化工行业", "industry")
            .with_aliases(&["化工行业"]),
    );
    idx.register(
        ConceptNode::new("423c1e9b75b415b4b6dfa4c4e65930e7", "化纤行业", "industry")
            .with_aliases(&["化纤行业"]),
    );
    idx.register(
        ConceptNode::new("c0c139446e8f3667cc6bd6f238f085a3", "医疗器械", "industry")
            .with_aliases(&["医疗器械"]),
    );
    idx.register(
        ConceptNode::new("7ec6cb11f432f686b06ee1d489a8e661", "印刷包装", "industry")
            .with_aliases(&["印刷包装"]),
    );
    idx.register(
        ConceptNode::new("e20136012e4e587299875664e60e6da3", "发电设备", "industry")
            .with_aliases(&["发电设备"]),
    );
    idx.register(
        ConceptNode::new("a2c6239da6b189ac41964eb35e313bb3", "商业百货", "industry")
            .with_aliases(&["商业百货"]),
    );
    idx.register(
        ConceptNode::new("c902e1e1b84021d8e212ea62582a21da", "塑料制品", "industry")
            .with_aliases(&["塑料制品"]),
    );
    idx.register(
        ConceptNode::new("7f267a38ab8553a88b16ca28ec7221a1", "家具行业", "industry")
            .with_aliases(&["家具行业"]),
    );
    idx.register(
        ConceptNode::new("632f70e6556ae0d5a30c9bf503a606d3", "家电行业", "industry")
            .with_aliases(&["家电行业"]),
    );
    idx.register(
        ConceptNode::new("d589cbc3f9b3415d251027c97e79bed0", "建筑建材", "industry")
            .with_aliases(&["建筑建材"]),
    );
    idx.register(
        ConceptNode::new("f737cacf14b28dabb3b5900e72a529d5", "开发区", "industry")
            .with_aliases(&["开发区"]),
    );
    idx.register(
        ConceptNode::new("d7d3b3a5c6cb5ce94b13e268dd387516", "房地产", "industry")
            .with_aliases(&["房地产"]),
    );
    idx.register(
        ConceptNode::new("fed19a817b3427b634cb351c0e073041", "摩托车", "industry")
            .with_aliases(&["摩托车"]),
    );
    idx.register(
        ConceptNode::new("2b8431da11a537353c997e9e6011e0d8", "有色金属", "industry")
            .with_aliases(&["有色金属"]),
    );
    idx.register(
        ConceptNode::new("81e0aba32c6036ce2941db357d06312b", "服装鞋类", "industry")
            .with_aliases(&["服装鞋类"]),
    );
    idx.register(
        ConceptNode::new("7cd8a1ce8859642ce471514c57ac247f", "机械行业", "industry")
            .with_aliases(&["机械行业"]),
    );
    idx.register(
        ConceptNode::new("50371a2c5078b757a8f8c75b8877e815", "次新股", "industry")
            .with_aliases(&["次新股"]),
    );
    idx.register(
        ConceptNode::new("b87afb222f06bf73c2576ce604c53fe5", "水泥行业", "industry")
            .with_aliases(&["水泥行业"]),
    );
    idx.register(
        ConceptNode::new("53142353a2f2ac118f76869027b3a514", "汽车制造", "industry")
            .with_aliases(&["汽车制造"]),
    );
    idx.register(
        ConceptNode::new("4db43c61dc4f4d7ed868a79c41879e3b", "煤炭行业", "industry")
            .with_aliases(&["煤炭行业"]),
    );
    idx.register(
        ConceptNode::new("b0a068e969f11cbd5759ed51fb74401b", "物资外贸", "industry")
            .with_aliases(&["物资外贸"]),
    );
    idx.register(
        ConceptNode::new("9249c32884eb8b7b480f74538b06ead6", "环保行业", "industry")
            .with_aliases(&["环保行业"]),
    );
    idx.register(
        ConceptNode::new("3159a3474b29d95f46fa54b4ed7387c2", "玻璃行业", "industry")
            .with_aliases(&["玻璃行业"]),
    );
    idx.register(
        ConceptNode::new("b694c3f20a63b7014fe0e558dd38f054", "生物制药", "industry")
            .with_aliases(&["生物制药"]),
    );
    idx.register(
        ConceptNode::new("307219289fdf40455243d0c76138de9d", "电力行业", "industry")
            .with_aliases(&["电力行业"]),
    );
    idx.register(
        ConceptNode::new("87d78f65cf25694da0b1ebe786638276", "电器行业", "industry")
            .with_aliases(&["电器行业"]),
    );
    idx.register(
        ConceptNode::new("41052b4325b2bd6a2b1d101241d26a50", "电子信息", "industry")
            .with_aliases(&["电子信息"]),
    );
    idx.register(
        ConceptNode::new("8b508121df448eadb76ab225f6c29fd2", "电子器件", "industry")
            .with_aliases(&["电子器件"]),
    );
    idx.register(
        ConceptNode::new("0f7ed1ccbe76b38983106a7c98354ba6", "石油行业", "industry")
            .with_aliases(&["石油行业"]),
    );
    idx.register(
        ConceptNode::new("9940380e2d9634fa08d6caf2586a4490", "纺织机械", "industry")
            .with_aliases(&["纺织机械"]),
    );
    idx.register(
        ConceptNode::new("bf697f588be3438f2bb12c976f767552", "纺织行业", "industry")
            .with_aliases(&["纺织行业"]),
    );
    idx.register(
        ConceptNode::new("dc6473ac96a77fe376fd7dd828d62993", "综合行业", "industry")
            .with_aliases(&["综合行业"]),
    );
    idx.register(
        ConceptNode::new("02b06573f6838d337a7a1443df1e852d", "船舶制造", "industry")
            .with_aliases(&["船舶制造"]),
    );
    idx.register(
        ConceptNode::new("85725fc6c7f59ab385983636897a4483", "造纸行业", "industry")
            .with_aliases(&["造纸行业"]),
    );
    idx.register(
        ConceptNode::new("c3d35cf86d012cca9b545367d2067e63", "酒店旅游", "industry")
            .with_aliases(&["酒店旅游"]),
    );
    idx.register(
        ConceptNode::new("7299e2e6ab239711375b776bae997c04", "酿酒行业", "industry")
            .with_aliases(&["酿酒行业"]),
    );
    idx.register(
        ConceptNode::new("cf0d1beced89cfe9a176b7536b34fd0c", "金融行业", "industry")
            .with_aliases(&["金融行业"]),
    );
    idx.register(
        ConceptNode::new("e8111d5fafa7ea9d0ae36df2ccf9ce44", "钢铁行业", "industry")
            .with_aliases(&["钢铁行业"]),
    );
    idx.register(
        ConceptNode::new("cb94db4902e33fbb2bb76e613e13642a", "陶瓷行业", "industry")
            .with_aliases(&["陶瓷行业"]),
    );
    idx.register(
        ConceptNode::new("903326efad586e169275cdcb1368a00e", "飞机制造", "industry")
            .with_aliases(&["飞机制造"]),
    );
    idx.register(
        ConceptNode::new("77d829a58943cb745bd0441a30a34063", "食品行业", "industry")
            .with_aliases(&["食品行业"]),
    );

    // === 概念 (163) ===
    idx.register(
        ConceptNode::new("0e09994456681fa2a869af4b9509b020", "3D打印", "concept")
            .with_aliases(&["3D打印"]),
    );
    idx.register(
        ConceptNode::new("0451c32f64fae5a31f25eb31458d54cb", "4G概念", "concept")
            .with_aliases(&["4G概念"]),
    );
    idx.register(
        ConceptNode::new("ff6670e10555d19e1f934a4c42a30f83", "5G概念", "concept")
            .with_aliases(&["5G概念"]),
    );
    idx.register(
        ConceptNode::new("6967f3489bf384bbd0b0fab6a0f656ba", "IPV6概念", "concept")
            .with_aliases(&["IPV6概念"]),
    );
    idx.register(
        ConceptNode::new("c54a122d80863560722148fd6159aa2f", "IP变现", "concept")
            .with_aliases(&["IP变现"]),
    );
    idx.register(
        ConceptNode::new("bafb352f6dc3a71593811475ce7a0843", "O2O模式", "concept")
            .with_aliases(&["O2O模式"]),
    );
    idx.register(
        ConceptNode::new("683d176f30188400af1a2a065ef32fdc", "QFII重仓", "concept")
            .with_aliases(&["QFII重仓"]),
    );
    idx.register(
        ConceptNode::new("fff2238549ab3875a2d97cf5a05a055c", "ST板块", "concept")
            .with_aliases(&["ST板块"]),
    );
    idx.register(
        ConceptNode::new("999278e00523daeadee721f708094efe", "三沙概念", "concept")
            .with_aliases(&["三沙概念"]),
    );
    idx.register(
        ConceptNode::new("2fbf052a92afe638a0c7f59af99370dc", "三网融合", "concept")
            .with_aliases(&["三网融合"]),
    );
    idx.register(
        ConceptNode::new("58b9340e6dda919d86641499a1b89134", "上海本地", "concept")
            .with_aliases(&["上海本地"]),
    );
    idx.register(
        ConceptNode::new("7c2f739a43eb7ddbb7e423afe299f0ca", "上海自贸", "concept")
            .with_aliases(&["上海自贸"]),
    );
    idx.register(
        ConceptNode::new("460bdfc561ff7968c4cf0c52b249c802", "业绩预升", "concept")
            .with_aliases(&["业绩预升"]),
    );
    idx.register(
        ConceptNode::new("da3e6d9371ed8a50bc279dc2b2329b2e", "业绩预降", "concept")
            .with_aliases(&["业绩预降"]),
    );
    idx.register(
        ConceptNode::new("9f57f162f3fcea4735c149b73b21fab8", "东亚自贸", "concept")
            .with_aliases(&["东亚自贸"]),
    );
    idx.register(
        ConceptNode::new("036d1cc9a6b256f1213a4b32bf53fbb8", "丝绸之路", "concept")
            .with_aliases(&["丝绸之路"]),
    );
    idx.register(
        ConceptNode::new("1fefd5a9127ae81cd9e10ebb95084366", "云计算", "concept")
            .with_aliases(&["云计算"]),
    );
    idx.register(
        ConceptNode::new("fd48f2d675a14efad41cf84cf6769ef1", "互联金融", "concept")
            .with_aliases(&["互联金融"]),
    );
    idx.register(
        ConceptNode::new("9fbf88dbf5ce3814c31cbbad33ea5557", "京津冀", "concept")
            .with_aliases(&["京津冀"]),
    );
    idx.register(
        ConceptNode::new("0bdc58f6dbc08bd1f3dfa240e8748770", "低碳经济", "concept")
            .with_aliases(&["低碳经济"]),
    );
    idx.register(
        ConceptNode::new("0bc4ae32291eba1cb2dab241be37e44f", "体育概念", "concept")
            .with_aliases(&["体育概念"]),
    );
    idx.register(
        ConceptNode::new("fa73d82cc3d1f88082b23e6d5a2e5329", "保险重仓", "concept")
            .with_aliases(&["保险重仓"]),
    );
    idx.register(
        ConceptNode::new("2cc73537fd422d27d2065a34927a740e", "保障房", "concept")
            .with_aliases(&["保障房"]),
    );
    idx.register(
        ConceptNode::new("b001745de17beb444c257843a2e958ee", "信息安全", "concept")
            .with_aliases(&["信息安全"]),
    );
    idx.register(
        ConceptNode::new("6c464cd4b3e291a6ba510f827ba1234e", "信托重仓", "concept")
            .with_aliases(&["信托重仓"]),
    );
    idx.register(
        ConceptNode::new("a21f03c49ac574d7905dfa85c6df93f5", "充电桩", "concept")
            .with_aliases(&["充电桩"]),
    );
    idx.register(
        ConceptNode::new("35f2901f7ffd039dbe77de602b3f64ba", "免疫治疗", "concept")
            .with_aliases(&["免疫治疗"]),
    );
    idx.register(
        ConceptNode::new("929f745a9ba51fd6bc5323426d132b2b", "养老概念", "concept")
            .with_aliases(&["养老概念"]),
    );
    idx.register(
        ConceptNode::new("e829dae81f76e5ad5296c46625b99083", "内贸规划", "concept")
            .with_aliases(&["内贸规划"]),
    );
    idx.register(
        ConceptNode::new("d5447959d167fdc050912e65deb04560", "军工航天", "concept")
            .with_aliases(&["军工航天"]),
    );
    idx.register(
        ConceptNode::new("b78efea77b2110eacccaff07d7918e7f", "军民融合", "concept")
            .with_aliases(&["军民融合"]),
    );
    idx.register(
        ConceptNode::new("40cbf6325e3cbb57d52c7806496791ad", "农村金融", "concept")
            .with_aliases(&["农村金融"]),
    );
    idx.register(
        ConceptNode::new("e338909d6f6d2f2744e09f147191bde4", "准ST股", "concept")
            .with_aliases(&["准ST股"]),
    );
    idx.register(
        ConceptNode::new("df31e20cf6acdcdefabf625891efdd21", "出口退税", "concept")
            .with_aliases(&["出口退税"]),
    );
    idx.register(
        ConceptNode::new("8fabe140309a958229d28635a452913e", "分拆上市", "concept")
            .with_aliases(&["分拆上市"]),
    );
    idx.register(
        ConceptNode::new("baf98313e290fa5e6bd2552f7f032b8d", "创投概念", "concept")
            .with_aliases(&["创投概念"]),
    );
    idx.register(
        ConceptNode::new("d2942e979c3afa41e5dbcecbc0420f91", "券商重仓", "concept")
            .with_aliases(&["券商重仓"]),
    );
    idx.register(
        ConceptNode::new("94e4c112ae450e834b2873096cecea86", "前海概念", "concept")
            .with_aliases(&["前海概念"]),
    );
    idx.register(
        ConceptNode::new("b35a3277936e55013f484b257283321d", "博彩概念", "concept")
            .with_aliases(&["博彩概念"]),
    );
    idx.register(
        ConceptNode::new("6dced030df4112cdd0a5ff1187a13dee", "卫星导航", "concept")
            .with_aliases(&["卫星导航"]),
    );
    idx.register(
        ConceptNode::new("ce495dad1f2e6e6a706ae3b0148920f3", "参股金融", "concept")
            .with_aliases(&["参股金融"]),
    );
    idx.register(
        ConceptNode::new("2fa5b64fa758a1ec30e73f2ca05fe74d", "可燃冰", "concept")
            .with_aliases(&["可燃冰"]),
    );
    idx.register(
        ConceptNode::new("5cb9e579caa843ba618dd50edb4fe15b", "含B股", "concept")
            .with_aliases(&["含B股"]),
    );
    idx.register(
        ConceptNode::new("2142b6e58568bd6ca5e74cb99cc15363", "含H股", "concept")
            .with_aliases(&["含H股"]),
    );
    idx.register(
        ConceptNode::new("9158984476b204d24fe65e79311d5c85", "含可转债", "concept")
            .with_aliases(&["含可转债"]),
    );
    idx.register(
        ConceptNode::new("f33f47fba2faf8f662aa683192460efc", "固废处理", "concept")
            .with_aliases(&["固废处理"]),
    );
    idx.register(
        ConceptNode::new("1d899a809c63e9e37ba1a245d1ef1703", "国产软件", "concept")
            .with_aliases(&["国产软件"]),
    );
    idx.register(
        ConceptNode::new("a3b9e433cfe44e82f8521415e4ebf25d", "国企改革", "concept")
            .with_aliases(&["国企改革"]),
    );
    idx.register(
        ConceptNode::new("43eed82bbe88fe78ed3b666fe142dc61", "图们江", "concept")
            .with_aliases(&["图们江"]),
    );
    idx.register(
        ConceptNode::new("b961f48b0704ee540ffa884f36336ced", "土地流转", "concept")
            .with_aliases(&["土地流转"]),
    );
    idx.register(
        ConceptNode::new("ae435415eac47a64976b18c748cfac9c", "地热能", "concept")
            .with_aliases(&["地热能"]),
    );
    idx.register(
        ConceptNode::new("4ed649869e68d3f02803413d12ddb14d", "基因概念", "concept")
            .with_aliases(&["基因概念"]),
    );
    idx.register(
        ConceptNode::new("02223b9967945188275694362fa74f6d", "基因测序", "concept")
            .with_aliases(&["基因测序"]),
    );
    idx.register(
        ConceptNode::new("c48f805b17a106b2d15876c0dc6064cd", "基因芯片", "concept")
            .with_aliases(&["基因芯片"]),
    );
    idx.register(
        ConceptNode::new("44ab7580dea7e843b7589c720b30606d", "基金重仓", "concept")
            .with_aliases(&["基金重仓"]),
    );
    idx.register(
        ConceptNode::new("5002146d64e919ca0d3af525a8b4083e", "外资背景", "concept")
            .with_aliases(&["外资背景"]),
    );
    idx.register(
        ConceptNode::new("d60521ee1c4e6cdd2097ac06a6d65766", "多晶硅", "concept")
            .with_aliases(&["多晶硅"]),
    );
    idx.register(
        ConceptNode::new("1690db91ffae6f6466c79b8e650af811", "天津自贸", "concept")
            .with_aliases(&["天津自贸"]),
    );
    idx.register(
        ConceptNode::new("2cc60d3b30e5ff15cc795f0b922c2bb2", "太阳能", "concept")
            .with_aliases(&["太阳能"]),
    );
    idx.register(
        ConceptNode::new("efd1fbf83a6605018a9c05f272ffdc48", "央企50", "concept")
            .with_aliases(&["央企50"]),
    );
    idx.register(
        ConceptNode::new("c50f4cbad6d435dc9b60ca09762a4eff", "奢侈品", "concept")
            .with_aliases(&["奢侈品"]),
    );
    idx.register(
        ConceptNode::new("5c52644dba5154611bc3921e2d96f9ed", "婴童概念", "concept")
            .with_aliases(&["婴童概念"]),
    );
    idx.register(
        ConceptNode::new("6e48f05289ce6e8e8bed2784bcc0c3db", "安防服务", "concept")
            .with_aliases(&["安防服务"]),
    );
    idx.register(
        ConceptNode::new("ad5b9491674a614982c7ce9d51bfe04c", "宽带提速", "concept")
            .with_aliases(&["宽带提速"]),
    );
    idx.register(
        ConceptNode::new("c3b38fd34d92c1c910fb48a6dd840a75", "广东自贸", "concept")
            .with_aliases(&["广东自贸"]),
    );
    idx.register(
        ConceptNode::new("02d62c41f0feca507e969765d52041ba", "建筑节能", "concept")
            .with_aliases(&["建筑节能"]),
    );
    idx.register(
        ConceptNode::new("d492fac24987b7c9be24698f7ecd9b72", "循环经济", "concept")
            .with_aliases(&["循环经济"]),
    );
    idx.register(
        ConceptNode::new("75e96079eda9b8f8ff26ec4d25a0dfc3", "成渝特区", "concept")
            .with_aliases(&["成渝特区"]),
    );
    idx.register(
        ConceptNode::new("9c615f7729db37e79cd1a65fd90a24c2", "抗流感", "concept")
            .with_aliases(&["抗流感"]),
    );
    idx.register(
        ConceptNode::new("2960a154b9c0ba04b74b2100f2b18277", "抗癌", "concept")
            .with_aliases(&["抗癌"]),
    );
    idx.register(
        ConceptNode::new("fa74414e38bafc5096d3910564389c20", "振兴沈阳", "concept")
            .with_aliases(&["振兴沈阳"]),
    );
    idx.register(
        ConceptNode::new("7367c9a6d22d840fed391e01da06a27b", "摘帽概念", "concept")
            .with_aliases(&["摘帽概念"]),
    );
    idx.register(
        ConceptNode::new("7467e74ead20cec2dd5fa3c9b5cda8ad", "整体上市", "concept")
            .with_aliases(&["整体上市"]),
    );
    idx.register(
        ConceptNode::new("d5243e0eb9d62a894a3457338d92c042", "文化振兴", "concept")
            .with_aliases(&["文化振兴"]),
    );
    idx.register(
        ConceptNode::new("157886f7112f86abbd9db676ce3b3922", "新三板", "concept")
            .with_aliases(&["新三板"]),
    );
    idx.register(
        ConceptNode::new("8a68be7b3f4fdaa74c280704fa9edf50", "新能源", "concept")
            .with_aliases(&["新能源"]),
    );
    idx.register(
        ConceptNode::new("6dbb02438f4de77850c3e547bb4dc5d6", "新零售", "concept")
            .with_aliases(&["新零售"]),
    );
    idx.register(
        ConceptNode::new("91edcee752394b1d27830edd49b4202c", "日韩贸易", "concept")
            .with_aliases(&["日韩贸易"]),
    );
    idx.register(
        ConceptNode::new("fd58aced82a63e190448b8bb7fd64743", "智能交通", "concept")
            .with_aliases(&["智能交通"]),
    );
    idx.register(
        ConceptNode::new("1fc54501b18c13a5fdf42b6b9280237f", "智能家居", "concept")
            .with_aliases(&["智能家居"]),
    );
    idx.register(
        ConceptNode::new("de449579cee1a29e6b5826fe7d7bd280", "智能机器", "concept")
            .with_aliases(&["智能机器"]),
    );
    idx.register(
        ConceptNode::new("0a53d6bc51e51bc22b044202e9c96a2b", "智能电网", "concept")
            .with_aliases(&["智能电网"]),
    );
    idx.register(
        ConceptNode::new("fcc55f621426b2c8d8b08430f2e6c946", "智能穿戴", "concept")
            .with_aliases(&["智能穿戴"]),
    );
    idx.register(
        ConceptNode::new("5183c2a5461daf0ce69422e29fcb983c", "未股改", "concept")
            .with_aliases(&["未股改"]),
    );
    idx.register(
        ConceptNode::new("6f13e085fd060b7608d311852abb7d51", "本月解禁", "concept")
            .with_aliases(&["本月解禁"]),
    );
    idx.register(
        ConceptNode::new("56958bad389223df85fec1c62bf23c6f", "机器人概念", "concept")
            .with_aliases(&["机器人概念"]),
    );
    idx.register(
        ConceptNode::new("cc1340316e41046e7ff2f54c33418694", "核电核能", "concept")
            .with_aliases(&["核电核能"]),
    );
    idx.register(
        ConceptNode::new("50371a2c5078b757a8f8c75b8877e815", "次新股", "concept")
            .with_aliases(&["次新股"]),
    );
    idx.register(
        ConceptNode::new("013a7f5056ed91e71b87cfff322c9098", "武汉规划", "concept")
            .with_aliases(&["武汉规划"]),
    );
    idx.register(
        ConceptNode::new("0dd0387512e711fa61663c26b92781cc", "民营医院", "concept")
            .with_aliases(&["民营医院"]),
    );
    idx.register(
        ConceptNode::new("f04fb8b2849b70f20d5f44fb9ee51a83", "民营银行", "concept")
            .with_aliases(&["民营银行"]),
    );
    idx.register(
        ConceptNode::new("83ee0bcb7f4d04367e8834b88995bd8a", "氢燃料", "concept")
            .with_aliases(&["氢燃料"]),
    );
    idx.register(
        ConceptNode::new("cde37b964aa9892b8e74cc17acfcdab9", "水利建设", "concept")
            .with_aliases(&["水利建设"]),
    );
    idx.register(
        ConceptNode::new("939bc595fa136d136a570c15e2db62b4", "水域改革", "concept")
            .with_aliases(&["水域改革"]),
    );
    idx.register(
        ConceptNode::new("c43e7a135c81d6b336d709a3bf240856", "污水处理", "concept")
            .with_aliases(&["污水处理"]),
    );
    idx.register(
        ConceptNode::new("3460789fdf46889dadd5ae385b9c3e0e", "汽车电子", "concept")
            .with_aliases(&["汽车电子"]),
    );
    idx.register(
        ConceptNode::new("5ff641ea9d2f4a1577b783a6b28faedc", "油气改革", "concept")
            .with_aliases(&["油气改革"]),
    );
    idx.register(
        ConceptNode::new("54dd32b6b7ebf0da5ddd5c467210cabc", "沿海发展", "concept")
            .with_aliases(&["沿海发展"]),
    );
    idx.register(
        ConceptNode::new("e1731903a577ec837092ce9e42ed3f3f", "海上丝路", "concept")
            .with_aliases(&["海上丝路"]),
    );
    idx.register(
        ConceptNode::new("fa5eb15ffb035857484b59d587c4c896", "海峡西岸", "concept")
            .with_aliases(&["海峡西岸"]),
    );
    idx.register(
        ConceptNode::new("c92028bb82a69ab273da55e4a3d82907", "海工装备", "concept")
            .with_aliases(&["海工装备"]),
    );
    idx.register(
        ConceptNode::new("89e65d47694a5405b200a666d3a01dcb", "海水淡化", "concept")
            .with_aliases(&["海水淡化"]),
    );
    idx.register(
        ConceptNode::new("7f65d914deb3012ab527bf422129bbb3", "涉矿概念", "concept")
            .with_aliases(&["涉矿概念"]),
    );
    idx.register(
        ConceptNode::new("4402abf98a55ab638247dc611b8c9c9c", "深圳本地", "concept")
            .with_aliases(&["深圳本地"]),
    );
    idx.register(
        ConceptNode::new("753778ec03af791329e9765babe908d6", "燃料电池", "concept")
            .with_aliases(&["燃料电池"]),
    );
    idx.register(
        ConceptNode::new("3d13e1c086cc2bde080a9fdd36076454", "物联网", "concept")
            .with_aliases(&["物联网"]),
    );
    idx.register(
        ConceptNode::new("9edef009c26359c7af8f5215a218043d", "特斯拉", "concept")
            .with_aliases(&["特斯拉"]),
    );
    idx.register(
        ConceptNode::new("b26d26794a5ab0725efb4cbe0f9f8d8d", "猪肉", "concept")
            .with_aliases(&["猪肉"]),
    );
    idx.register(
        ConceptNode::new("4cdc41aac1ae49be2e0daa3c9340885e", "生态农业", "concept")
            .with_aliases(&["生态农业"]),
    );
    idx.register(
        ConceptNode::new("4ac2e25e32efdcd2059be0a56020e36c", "生物燃料", "concept")
            .with_aliases(&["生物燃料"]),
    );
    idx.register(
        ConceptNode::new("c342c2df1911944687afef90d8b79dd7", "生物疫苗", "concept")
            .with_aliases(&["生物疫苗"]),
    );
    idx.register(
        ConceptNode::new("427aa8e005adf9f454f3da4218d0da2a", "生物育种", "concept")
            .with_aliases(&["生物育种"]),
    );
    idx.register(
        ConceptNode::new("991345c4d0f47afc5701f17b7a22b83e", "生物质能", "concept")
            .with_aliases(&["生物质能"]),
    );
    idx.register(
        ConceptNode::new("c97520c78b10b1ee35b289dd6af390d1", "甲型流感", "concept")
            .with_aliases(&["甲型流感"]),
    );
    idx.register(
        ConceptNode::new("cc91e6bdb59f2e03ee24d7b6439f1015", "电商概念", "concept")
            .with_aliases(&["电商概念"]),
    );
    idx.register(
        ConceptNode::new("7a0bcf5143c6f33f0fde4e4cb4904447", "电子支付", "concept")
            .with_aliases(&["电子支付"]),
    );
    idx.register(
        ConceptNode::new("ce3694bc812e5d7c587d98206fe9a7f5", "皖江区域", "concept")
            .with_aliases(&["皖江区域"]),
    );
    idx.register(
        ConceptNode::new("5dd6433d1821ca7c4b1b70eda21c5b61", "石墨烯", "concept")
            .with_aliases(&["石墨烯"]),
    );
    idx.register(
        ConceptNode::new("0955d7caf0605bc27b536694090c5151", "碳纤维", "concept")
            .with_aliases(&["碳纤维"]),
    );
    idx.register(
        ConceptNode::new("313e05f2d519a12df32d1ad1ed5e5157", "社保重仓", "concept")
            .with_aliases(&["社保重仓"]),
    );
    idx.register(
        ConceptNode::new("c43d84b74def4fb08c9d8f467d827038", "稀土永磁", "concept")
            .with_aliases(&["稀土永磁"]),
    );
    idx.register(
        ConceptNode::new("fad4ec6a0c7ba8e9121ca1cf302987e3", "稀缺资源", "concept")
            .with_aliases(&["稀缺资源"]),
    );
    idx.register(
        ConceptNode::new("fbe896126d879b73704fc67243a5e2d0", "空气治理", "concept")
            .with_aliases(&["空气治理"]),
    );
    idx.register(
        ConceptNode::new("d1fa280ecb16856f18e037b82ec67fd2", "粤港澳", "concept")
            .with_aliases(&["粤港澳"]),
    );
    idx.register(
        ConceptNode::new("5fb5391ad439c30c7c0bb349e557a098", "维生素", "concept")
            .with_aliases(&["维生素"]),
    );
    idx.register(
        ConceptNode::new("b335bf429dd7ce19b43da2a71b697cfb", "绿色照明", "concept")
            .with_aliases(&["绿色照明"]),
    );
    idx.register(
        ConceptNode::new("b9d954a0c6c49d8246cffd2490727f07", "网络游戏", "concept")
            .with_aliases(&["网络游戏"]),
    );
    idx.register(
        ConceptNode::new("e61cb49f86211e94e8c029c68b235510", "聚氨酯", "concept")
            .with_aliases(&["聚氨酯"]),
    );
    idx.register(
        ConceptNode::new("2190b07f8f8548930ba40ab8951ed23c", "股期概念", "concept")
            .with_aliases(&["股期概念"]),
    );
    idx.register(
        ConceptNode::new("4994afb7d1c473b7b6564d8a3dab2918", "股权激励", "concept")
            .with_aliases(&["股权激励"]),
    );
    idx.register(
        ConceptNode::new("8201dc776d9e6b16eeb042af6b225a57", "自贸区", "concept")
            .with_aliases(&["自贸区"]),
    );
    idx.register(
        ConceptNode::new("a3c16c6740036b2df94ddd4dc8c885c3", "节能", "concept")
            .with_aliases(&["节能"]),
    );
    idx.register(
        ConceptNode::new("db4ebeeec5f8f7acf6854e0d244b2432", "节能环保", "concept")
            .with_aliases(&["节能环保"]),
    );
    idx.register(
        ConceptNode::new("acf3d4990cc6e37a29ebe44007b906ab", "苹果概念", "concept")
            .with_aliases(&["苹果概念"]),
    );
    idx.register(
        ConceptNode::new("d801d304575a166575035eae21e207e2", "草甘膦", "concept")
            .with_aliases(&["草甘膦"]),
    );
    idx.register(
        ConceptNode::new("d58d230a44847977718b1bb2ffb4c8ce", "蓝宝石", "concept")
            .with_aliases(&["蓝宝石"]),
    );
    idx.register(
        ConceptNode::new("02e9a0d2773dbfac5b6db2d05e10176a", "融资融券", "concept")
            .with_aliases(&["融资融券"]),
    );
    idx.register(
        ConceptNode::new("96a22825c11cad73f1c1a8b2f8746b37", "装饰园林", "concept")
            .with_aliases(&["装饰园林"]),
    );
    idx.register(
        ConceptNode::new("f1507f026988192110407ff48aebefa9", "触摸屏", "concept")
            .with_aliases(&["触摸屏"]),
    );
    idx.register(
        ConceptNode::new("88b97891a47f08e14f27f9023d10487d", "资产注入", "concept")
            .with_aliases(&["资产注入"]),
    );
    idx.register(
        ConceptNode::new("9b0aa54ec004a91e34c58dfc2a43d7ca", "赛马概念", "concept")
            .with_aliases(&["赛马概念"]),
    );
    idx.register(
        ConceptNode::new("64882c9f4f3d5c2de2ade0d85f714f1a", "超大盘", "concept")
            .with_aliases(&["超大盘"]),
    );
    idx.register(
        ConceptNode::new("4bef93e1c13139d6888640474111b87e", "超导概念", "concept")
            .with_aliases(&["超导概念"]),
    );
    idx.register(
        ConceptNode::new("8ada12ee82976dcdbdd95ea777730fc0", "超级细菌", "concept")
            .with_aliases(&["超级细菌"]),
    );
    idx.register(
        ConceptNode::new("9d0ff7bb09c400a510be93c6cb752ada", "迪士尼", "concept")
            .with_aliases(&["迪士尼"]),
    );
    idx.register(
        ConceptNode::new("c28fed98c081eb3a66fe608d4353401f", "送转潜力", "concept")
            .with_aliases(&["送转潜力"]),
    );
    idx.register(
        ConceptNode::new("6d0f9112576fec89404d42503f9d3c12", "重组概念", "concept")
            .with_aliases(&["重组概念"]),
    );
    idx.register(
        ConceptNode::new("105a1be68034b2bd11c887fa1f257291", "金融参股", "concept")
            .with_aliases(&["金融参股"]),
    );
    idx.register(
        ConceptNode::new("47e7992062714856971eb347f409ab2c", "金融改革", "concept")
            .with_aliases(&["金融改革"]),
    );
    idx.register(
        ConceptNode::new("e2fef2c29267f1ab8f51271ce3017f93", "铁路基建", "concept")
            .with_aliases(&["铁路基建"]),
    );
    idx.register(
        ConceptNode::new("aa9f49a8233495bf8d244017ab01f079", "锂电池", "concept")
            .with_aliases(&["锂电池"]),
    );
    idx.register(
        ConceptNode::new("7300a38a34a150c5ac16ef45e875b1f5", "长株潭", "concept")
            .with_aliases(&["长株潭"]),
    );
    idx.register(
        ConceptNode::new("6d47760a599a239f4c11a174c0b14a63", "阿里概念", "concept")
            .with_aliases(&["阿里概念"]),
    );
    idx.register(
        ConceptNode::new("c8019a1f3d05ef35bed5c456458016b1", "陕甘宁", "concept")
            .with_aliases(&["陕甘宁"]),
    );
    idx.register(
        ConceptNode::new("3da066df02b56c45aab6e20e4e50d2d9", "雄安新区", "concept")
            .with_aliases(&["雄安新区"]),
    );
    idx.register(
        ConceptNode::new("e2f280b81cfa090f66c44cee2b6a6a3e", "页岩气", "concept")
            .with_aliases(&["页岩气"]),
    );
    idx.register(
        ConceptNode::new("6b6c7570c1f30212ac32e71ad106e6b7", "风沙治理", "concept")
            .with_aliases(&["风沙治理"]),
    );
    idx.register(
        ConceptNode::new("a5533e698a754c4f80868a67802e7ba0", "风能", "concept")
            .with_aliases(&["风能"]),
    );
    idx.register(
        ConceptNode::new("31d6f92ce29c9039c60177cfd2e843e2", "风能概念", "concept")
            .with_aliases(&["风能概念"]),
    );
    idx.register(
        ConceptNode::new("0e8032dff9c1bcfdc190025e7b5fb547", "食品安全", "concept")
            .with_aliases(&["食品安全"]),
    );
    idx.register(
        ConceptNode::new("3f5d2aefbd0b8662d071b9a32dae812c", "高校背景", "concept")
            .with_aliases(&["高校背景"]),
    );
    idx.register(
        ConceptNode::new("8e856b2badde34447c485619e2a3f701", "黄河三角", "concept")
            .with_aliases(&["黄河三角"]),
    );
    idx.register(
        ConceptNode::new("b356e44e78cfb260e8993e1dc0bde2f6", "黄金概念", "concept")
            .with_aliases(&["黄金概念"]),
    );
}

/// 用 `knowledge-sources/sample/edges.csv` 的内容构造一个可直接用于测试的索引
pub fn build_sample_index() -> ConceptIndex {
    let edges = SAMPLE_EDGES
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let p: Vec<&str> = l.split(',').collect();
            (p[0].to_string(), p[1].to_string(), p[2].to_string())
        })
        .collect::<Vec<_>>();
    let mut idx = ConceptIndex::from_graph_edges(&edges);
    seed_sample_ontology(&mut idx);
    idx
}

/// 与 `knowledge-sources/sample/edges.csv` 保持一致（19 条边，去除非成员边）
const SAMPLE_EDGES: &str = r#"
000001,industry_bank,in_industry
600036,industry_bank,in_industry
601398,industry_bank,in_industry
601318,industry_insurance,in_industry
601628,industry_insurance,in_industry
688981,industry_semiconductor,in_industry
603501,industry_semiconductor,in_industry
002415,industry_semiconductor,in_industry
688981,concept_chip,has_concept
603501,concept_chip,has_concept
002415,concept_ai,has_concept
688981,concept_ai,has_concept
601318,concept_ai,has_concept
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_graph_edges_builds_membership() {
        let idx = build_sample_index();
        let ai = idx.members("concept_ai");
        assert_eq!(ai.len(), 3);
        assert!(ai.contains("002415"));
        assert!(ai.contains("688981"));
        assert!(ai.contains("601318"));

        let semi = idx.members("industry_semiconductor");
        assert_eq!(semi.len(), 3);
        assert!(semi.contains("002415"));
        assert!(semi.contains("688981"));
        assert!(semi.contains("603501"));
    }

    #[test]
    fn ontology_resolve_aliases() {
        let idx = build_sample_index();
        assert_eq!(idx.resolve("AI"), Some("concept_ai"));
        assert_eq!(idx.resolve("人工智能"), Some("concept_ai"));
        assert_eq!(idx.resolve("ai概念"), Some("concept_ai"));
        assert_eq!(idx.resolve("银行"), Some("industry_bank"));
        assert_eq!(idx.resolve("保险"), Some("industry_insurance"));
        // 未注册的概念解析失败
        assert_eq!(idx.resolve("新能源"), None);
    }

    #[test]
    fn theme_universe_or_vs_and() {
        let idx = build_sample_index();
        // OR：AI 概念 ∪ 半导体行业 = {002415,688981,601318,603501}
        let or = idx.theme_universe(&["AI".to_string(), "半导体".to_string()], false);
        assert_eq!(or.len(), 4);
        assert!(or.contains("002415"));
        assert!(or.contains("603501"));

        // AND：AI 概念 ∩ 半导体行业 = {002415,688981}（两只同时是 AI 概念且属半导体）
        let and = idx.theme_universe(&["AI".to_string(), "半导体".to_string()], true);
        assert_eq!(and.len(), 2);
        assert!(and.contains("002415"));
        assert!(and.contains("688981"));
        assert!(!and.contains("601318")); // 601318 属保险，非半导体
    }
}
