//! 模组中文名与中文搜索词。
//!
//! Modrinth 的 `title` / `description` 只有英文，也没有 i18n 字段，所以这里自带两张小表：
//! - [`title_zh`]：slug / 英文名 → 中文名（只收热门项目，查不到返回 None，界面退回英文）
//! - [`translate_query`]：中文关键词 → 英文关键词（用户输入「钠」时按 sodium 去查）
//!
//! 两张表都只是「够用」的规模，后续补词直接往下加。

/// 热门项目的中文名。key 用 Modrinth 的 slug（全小写），值 = 界面显示的中文名。
const NAMES: &[(&str, &str)] = &[
    // 优化
    ("sodium", "钠"),
    ("sodium-extra", "钠·扩展"),
    ("lithium", "锂"),
    ("phosphor", "磷"),
    ("krypton", "氪"),
    ("starlight", "星光"),
    ("lazydfu", "懒 DFU"),
    ("ferritecore", "铁氧体磁芯"),
    ("memoryleakfix", "内存泄漏修复"),
    ("modernfix", "现代修复"),
    ("entityculling", "实体渲染优化"),
    ("immediatelyfast", "即时渲染加速"),
    ("embeddium", "内嵌式渲染"),
    ("oculus", "视觉"),
    ("iris", "虹膜"),
    ("nvidium", "Nvidium"),
    ("c2me-fabric", "C2ME 并发区块"),
    ("noisium", "噪声优化"),
    ("clumps", "经验合并"),
    ("debugify", "调试修复"),
    ("bobby", "Bobby 远距离渲染"),
    ("distanthorizons", "遥远的地平线"),
    ("chunky", "区块预生成"),
    // 基础库
    ("fabric-api", "Fabric API"),
    ("architectury-api", "Architectury API"),
    ("cloth-config", "Cloth Config"),
    ("modmenu", "模组菜单"),
    ("fabric-language-kotlin", "Fabric Kotlin 支持"),
    ("forge-config-api-port", "Forge 配置 API 移植"),
    ("balm", "Balm"),
    ("kotlinforforge", "Kotlin for Forge"),
    ("geckolib", "GeckoLib 动画库"),
    ("cit-resewn", "CIT Resewn"),
    ("indium", "铟"),
    ("quilted-fabric-api", "Quilt 版 Fabric API"),
    // 玩法
    ("create", "机械动力"),
    ("ae2", "应用能源 2"),
    ("refined-storage", "精致存储"),
    ("mekanism", "通用机械"),
    ("thermal-expansion", "热力膨胀"),
    ("immersive-engineering", "沉浸工程"),
    ("botania", "植物魔法"),
    ("blood-magic", "血魔法"),
    ("ars-nouveau", "新生魔艺"),
    ("twilight-forest", "暮色森林"),
    ("the-aether", "天境"),
    ("biomes-o-plenty", "超多生物群系"),
    ("terralith", "Terralith 地形"),
    ("tectonic", "地形构造"),
    ("structory", "Structory 结构"),
    ("betterend", "更好的末地"),
    ("betternether", "更好的下界"),
    ("farmer's-delight", "农夫乐事"),
    ("croptopia", "作物盛景"),
    ("quark", "夸克"),
    ("supplementaries", "附属"),
    ("comforts", "舒适"),
    ("carry-on", "搬运"),
    ("corpse", "尸体"),
    ("gravestone-mod", "墓碑"),
    ("waystones", "传送石"),
    ("sophisticated-backpacks", "精致背包"),
    ("journeymap", "旅行地图"),
    ("xaeros-minimap", "Xaero 小地图"),
    ("xaeros-world-map", "Xaero 世界地图"),
    ("appleskin", "苹果皮"),
    ("jade", "玉 Jade"),
    ("wthit", "这是什么"),
    ("jei", "物品管理器 JEI"),
    ("roughly-enough-items", "物品管理器 REI"),
    ("emi", "EMI 物品管理器"),
    ("worldedit", "创世神 WorldEdit"),
    ("litematica", "投影 Litematica"),
    ("minihud", "MiniHUD"),
    ("carpet", "地毯"),
    ("lightmatica", "投影辅助"),
];


/// 词级术语表：给精确表里没有的项目「拼」出中文名。
///
/// 键是小写英文单词（或短语），值是中文；未收录的词原样保留，所以结果可能是中英混排。
const TERMS: &[(&str, &str)] = &[
    // 通用后缀 / 修饰
    ("extra", "扩展"),
    ("options", "选项"),
    ("settings", "设置"),
    ("config", "配置"),
    ("configuration", "配置"),
    ("api", "API"),
    ("lib", "库"),
    ("library", "库"),
    ("core", "核心"),
    ("utils", "工具"),
    ("utility", "实用工具"),
    ("tools", "工具"),
    ("client", "客户端"),
    ("server", "服务器"),
    ("mod", "模组"),
    ("mods", "模组"),
    ("plus", "扩展"),
    ("expanded", "扩展"),
    ("better", "更好的"),
    ("improved", "改进的"),
    ("enhanced", "增强"),
    ("advanced", "高级"),
    ("ultimate", "终极"),
    ("simple", "简单"),
    ("easy", "简易"),
    ("fast", "快速"),
    ("quick", "快捷"),
    ("faster", "更快"),
    ("smooth", "平滑"),
    ("modern", "现代"),
    ("classic", "经典"),
    ("legacy", "经典版"),
    ("lite", "精简版"),
    ("mini", "迷你"),
    ("reloaded", "重载版"),
    ("remastered", "重制版"),
    ("reimagined", "重制版"),
    ("fixed", "修复版"),
    ("fix", "修复"),
    ("compat", "兼容"),
    ("compatibility", "兼容"),
    ("integration", "联动"),
    ("addon", "附加"),
    ("addons", "附加"),
    ("edition", "版"),
    ("pack", "包"),
    ("resource pack", "材质包"),
    // 画面 / 性能
    ("performance", "性能"),
    ("optimization", "优化"),
    ("optimizations", "优化"),
    ("optimized", "优化"),
    ("fps", "帧率"),
    ("render", "渲染"),
    ("rendering", "渲染"),
    ("graphics", "画面"),
    ("visual", "视觉"),
    ("visuals", "视觉"),
    ("visual", "视觉"),
    ("effects", "效果"),
    ("effect", "效果"),
    ("lighting", "光照"),
    ("light", "光照"),
    ("shadow", "阴影"),
    ("shadowy", "阴影"),
    ("shadows", "阴影"),
    ("reflection", "反射"),
    ("reflections", "反射"),
    ("bloom", "泛光"),
    ("sky", "天空"),
    ("clouds", "云"),
    ("fog", "雾"),
    ("stars", "星空"),
    ("shader", "光影"),
    ("shaders", "光影"),
    ("texture", "材质"),
    ("textures", "材质"),
    ("animation", "动画"),
    ("animations", "动画"),
    ("model", "模型"),
    ("models", "模型"),
    ("particles", "粒子"),
    ("entity", "实体"),
    ("entities", "实体"),
    ("culling", "剔除"),
    ("occlusion", "遮挡"),
    ("zoom", "缩放"),
    ("screenshot", "截图"),
    ("replay", "回放"),
    // 玩法
    ("world", "世界"),
    ("worldgen", "世界生成"),
    ("generation", "生成"),
    ("terrain", "地形"),
    ("biome", "生物群系"),
    ("biomes", "生物群系"),
    ("structure", "结构"),
    ("structures", "结构"),
    ("dungeon", "地牢"),
    ("dimension", "维度"),
    ("dimensions", "维度"),
    ("cave", "洞穴"),
    ("caves", "洞穴"),
    ("ore", "矿石"),
    ("ores", "矿石"),
    ("mining", "挖矿"),
    ("farming", "农业"),
    ("food", "食物"),
    ("cooking", "烹饪"),
    ("animal", "动物"),
    ("animals", "动物"),
    ("mob", "生物"),
    ("mobs", "生物"),
    ("monster", "怪物"),
    ("boss", "首领"),
    ("weapon", "武器"),
    ("weapons", "武器"),
    ("armor", "护甲"),
    ("equipment", "装备"),
    ("enchant", "附魔"),
    ("enchanting", "附魔"),
    ("enchantments", "附魔"),
    ("magic", "魔法"),
    ("spell", "法术"),
    ("spells", "法术"),
    ("technology", "科技"),
    ("tech", "科技"),
    ("machine", "机器"),
    ("machines", "机器"),
    ("energy", "能源"),
    ("power", "能量"),
    ("pipe", "管道"),
    ("pipes", "管道"),
    ("fluid", "流体"),
    ("fluids", "流体"),
    ("storage", "存储"),
    ("backpack", "背包"),
    ("backpacks", "背包"),
    ("inventory", "背包"),
    ("quest", "任务"),
    ("quests", "任务"),
    ("map", "地图"),
    ("maps", "地图"),
    ("minimap", "小地图"),
    ("waypoint", "路径点"),
    ("waypoints", "路径点"),
    ("teleport", "传送"),
    ("spawn", "出生点"),
    ("seed", "种子"),
    ("chunk", "区块"),
    ("chunks", "区块"),
    ("block", "方块"),
    ("blocks", "方块"),
    ("item", "物品"),
    ("items", "物品"),
    ("recipe", "配方"),
    ("recipes", "配方"),
    ("crafting", "合成"),
    ("ui", "界面"),
    ("menu", "菜单"),
    ("screen", "界面"),
    ("hud", "HUD"),
    ("chat", "聊天"),
    ("sound", "声音"),
    ("music", "音乐"),
    ("weather", "天气"),
    ("seasons", "季节"),
    ("time", "时间"),
    ("clock", "时钟"),
    ("coordinates", "坐标"),
    ("compass", "指南针"),
    ("path", "路径"),
    ("roads", "道路"),
    ("building", "建筑"),
    ("decoration", "装饰"),
    ("adventure", "冒险"),
    ("survival", "生存"),
    ("creative", "创造"),
    ("debug", "调试"),
    ("device", "设备"),
    ("check", "检测"),
    ("information", "信息"),
    ("info", "信息"),
    ("details", "详情"),
    ("status", "状态"),
    ("notification", "通知"),
    ("notifications", "通知"),
    ("fullbright", "全亮"),
    ("brightness", "亮度"),
    ("dark", "黑暗"),
    ("night", "夜晚"),
    ("day", "白天"),
    ("dynamic", "动态"),
    ("smart", "智能"),
    ("auto", "自动"),
    ("custom", "自定义"),
    ("portable", "便携"),
    ("lightweight", "轻量"),
    ("memory", "内存"),
    ("thread", "线程"),
    ("threads", "线程"),
    ("async", "异步"),
    ("parallel", "并行"),
    ("tick", "刻"),
    ("lag", "卡顿"),
    ("stutter", "卡顿"),
    // 常见专名（避免拼成奇怪结果）
    ("sodium", "钠"),
    ("embeddium", "内嵌式渲染"),
    ("iris", "虹膜"),
    ("oculus", "视觉"),
    ("lithium", "锂"),
    ("krypton", "氪"),
    ("phosphor", "磷"),
    ("starlight", "星光"),
    ("fabric", "Fabric"),
    ("forge", "Forge"),
    ("neoforge", "NeoForge"),
    ("quilt", "Quilt"),
    ("optifine", "高清修复"),
    ("optimizer", "优化器"),
    ("particle", "粒子"),
    ("noise", "噪声"),
    ("crystal", "水晶"),
    ("rail", "铁轨"),
    ("rails", "铁轨"),
    ("bad", "糟糕"),
    ("blockentity", "方块实体"),
    ("blockentities", "方块实体"),
    ("framerate", "帧率"),
    ("hitbox", "碰撞箱"),
    ("hitboxes", "碰撞箱"),
    ("lightmap", "光照贴图"),
    ("frustum", "视锥"),
    ("culling", "剔除"),
];

/// 中文关键词 → 英文关键词。按 key 长度从长到短匹配，避免「小地图」被「地图」抢先替换。
const WORDS: &[(&str, &str)] = &[
    ("小地图", "minimap"),
    ("世界地图", "world map"),
    ("物品管理器", "recipe viewer"),
    ("区块预生成", "chunk pregenerator"),
    ("实体剔除", "entity culling"),
    ("内存泄漏", "memory leak"),
    ("光影", "shader"),
    ("材质", "resource pack"),
    ("优化", "optimization"),
    ("性能", "performance"),
    ("加速", "boost"),
    ("渲染", "rendering"),
    ("建筑", "building"),
    ("魔法", "magic"),
    ("科技", "technology"),
    ("存储", "storage"),
    ("地图", "map"),
    ("界面", "ui"),
    ("装饰", "decoration"),
    ("冒险", "adventure"),
    ("工具", "utility"),
    ("食物", "food"),
    ("农业", "farming"),
    ("生存", "survival"),
    ("生物", "mob"),
    ("维度", "dimension"),
    ("任务", "quest"),
    ("背包", "inventory"),
    ("合成", "crafting"),
    ("附魔", "enchanting"),
    ("战斗", "combat"),
    ("传送", "teleport"),
    ("地形", "terrain"),
    ("世界", "world"),
    ("旅行", "journey"),
    ("钠", "sodium"),
    ("锂", "lithium"),
    ("氪", "krypton"),
    ("磷", "phosphor"),
    ("虹膜", "iris"),
    ("机械动力", "create"),
];

/// 项目的中文名；查不到返回 None（界面显示英文原名）。
pub fn title_zh(slug: &str, title: &str) -> Option<String> {
    let slug = slug.to_ascii_lowercase();
    // 1) 精确表
    if let Some((_, zh)) = NAMES.iter().find(|(key, _)| *key == slug) {
        return Some((*zh).to_string());
    }
    let title_lower = title.to_ascii_lowercase();
    if let Some((_, zh)) = NAMES
        .iter()
        .find(|(key, _)| title_lower.replace(' ', "-") == *key)
    {
        return Some((*zh).to_string());
    }
    // 2) 词级拼装：至少命中一个词才给结果，否则保留英文
    compose(title)
}

/// 把英文标题按术语表逐词换成中文。全都不认识时返回 None（界面继续显示英文）。
fn compose(title: &str) -> Option<String> {
    let mut out = String::new();
    let mut hit = false;
    // 括号里的缩写（SSPB）别翻，原样留住
    for raw in title.split_whitespace() {
        let (word, tail) = split_tail(raw);
        // CamelCase 拆词：BadOptimizations → Bad + Optimizations
        for piece in split_camel(word) {
            let key = piece.to_ascii_lowercase();
            let zh = TERMS.iter().find(|(k, _)| *k == key).map(|(_, v)| *v);
            match zh {
                Some(v) => {
                    hit = true;
                    // 前面是英文单词时补空格，避免「Reese's钠选项」这种粘连
                    if out.chars().last().map(|c| c.is_ascii_alphanumeric()).unwrap_or(false) {
                        out.push(' ');
                    }
                    out.push_str(v);
                }
                None => {
                    if !out.is_empty() && !out.ends_with(' ') {
                        out.push(' ');
                    }
                    out.push_str(piece);
                }
            }
        }
        out.push_str(tail);
    }
    if !hit {
        return None;
    }
    Some(out.trim().to_string())
}

/// CamelCase 拆词：`BadOptimizations` → [`Bad`, `Optimizations`]。
/// 全大写缩写（`JEI`、`SSPB`）保持完整，不拆。
fn split_camel(word: &str) -> Vec<&str> {
    let chars: Vec<(usize, char)> = word.char_indices().collect();
    let mut cuts = Vec::new();
    for (idx, (byte, ch)) in chars.iter().enumerate() {
        if idx == 0 || !ch.is_ascii_uppercase() {
            continue;
        }
        let prev = chars[idx - 1].1;
        let next_is_lower = chars.get(idx + 1).map(|(_, c)| c.is_ascii_lowercase()).unwrap_or(false);
        if prev.is_ascii_lowercase() || prev.is_ascii_digit() || (prev.is_ascii_uppercase() && next_is_lower) {
            cuts.push(*byte);
        }
    }
    if cuts.is_empty() {
        return vec![word];
    }
    let mut out = Vec::new();
    let mut start = 0;
    for cut in cuts {
        out.push(&word[start..cut]);
        start = cut;
    }
    out.push(&word[start..]);
    out.retain(|w| !w.is_empty());
    out
}

/// 拆出英文单词尾部的标点 / 括号内容，例如 "Blocks(SSPB)" → ("Blocks", "(SSPB)")
fn split_tail(raw: &str) -> (&str, &str) {
    if let Some(pos) = raw.find(|c: char| !c.is_ascii_alphanumeric() && c != '\'') {
        raw.split_at(pos)
    } else {
        (raw, "")
    }
}

/// 中文查询词 → 英文查询词。没有中文命中时原样返回。
pub fn translate_query(query: &str) -> String {
    let has_cjk = query.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c));
    if !has_cjk {
        return query.to_string();
    }

    let mut text = query.to_string();
    let mut hit = false;
    // 长的先替换，避免短词把长词拆开
    let mut words: Vec<&(&str, &str)> = WORDS.iter().collect();
    words.sort_by_key(|(key, _)| std::cmp::Reverse(key.chars().count()));
    for (zh, en) in words {
        if text.contains(zh) {
            text = text.replace(zh, en);
            hit = true;
        }
    }
    if !hit {
        return query.to_string();
    }
    // 换行/多余空格收拾一下
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translates_common_words() {
        assert_eq!(translate_query("钠"), "sodium");
        assert_eq!(translate_query("优化"), "optimization");
        assert_eq!(translate_query("小地图"), "minimap");
        // 长词优先：小地图 不该变成 "小map"
        assert_eq!(translate_query("小地图 优化"), "minimap optimization");
        // 英文原样返回
        assert_eq!(translate_query("sodium"), "sodium");
        // 查不到的中文别乱翻
        assert_eq!(translate_query("找不到的词"), "找不到的词");
    }

    #[test]
    fn composes_titles_from_terms() {
        // 没有精确条目时按词拼；未收录的专名保留原样
        let zh = title_zh("sodium-shadowy-path-blocks", "Sodium Shadowy Path Blocks").unwrap();
        assert_eq!(zh, "钠阴影路径方块");
        let zh = title_zh("sodium-extra-information", "Sodium Extra Information").unwrap();
        assert_eq!(zh, "钠扩展信息");
        // CamelCase 也要能拆开
        assert_eq!(
            title_zh("badoptimizations", "BadOptimizations").as_deref(),
            Some("糟糕优化")
        );
        assert_eq!(
            title_zh("particlecore", "ParticleCore").as_deref(),
            Some("粒子核心")
        );
        // 一个词都不认识 → 不给中文名
        assert!(title_zh("unknown-thing", "Zorblax Frobnicator").is_none());
    }

    #[test]
    fn looks_up_chinese_titles() {
        assert_eq!(title_zh("sodium", "Sodium").as_deref(), Some("钠"));
        assert_eq!(title_zh("unknown-slug", "Whatever"), None);
        assert_eq!(title_zh("JEI", "Just Enough Items").as_deref(), Some("物品管理器 JEI"));
    }
}
