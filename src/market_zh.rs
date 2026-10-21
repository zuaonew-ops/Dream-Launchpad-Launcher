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
    if let Some((_, zh)) = NAMES.iter().find(|(key, _)| *key == slug) {
        return Some((*zh).to_string());
    }
    // slug 对不上时按英文名兜底（大小写、空格差异）
    let title_lower = title.to_ascii_lowercase();
    NAMES
        .iter()
        .find(|(key, _)| title_lower.replace(' ', "-") == *key)
        .map(|(_, zh)| (*zh).to_string())
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
    fn looks_up_chinese_titles() {
        assert_eq!(title_zh("sodium", "Sodium").as_deref(), Some("钠"));
        assert_eq!(title_zh("unknown-slug", "Whatever"), None);
        assert_eq!(title_zh("JEI", "Just Enough Items").as_deref(), Some("物品管理器 JEI"));
    }
}
