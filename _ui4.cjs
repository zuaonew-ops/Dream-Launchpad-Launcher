const fs = require("fs");

// 1) 版本行：正式版不再挂徽章，只有快照 / 本地 / 愚人节才标
let v = fs.readFileSync("ui/src/components/VersionList.tsx", "utf8");
v = v.replace(
  `                  <span className={version.kind === "release" ? "kind" : "kind snapshot"}>
                    {version.kind === "release" ? "正式" : version.kind === "snapshot" ? "快照" : "其它"}
                  </span>`,
  `                  {version.kind !== "release" && (
                    <span className="kind snapshot">
                      {version.kind === "snapshot" ? "快照" : "其它"}
                    </span>
                  )}`
);
fs.writeFileSync("ui/src/components/VersionList.tsx", v);

// 2) 内容页描述重复版本号
let c = fs.readFileSync("ui/src/pages/ContentPage.tsx", "utf8");
c = c.replace(
  `            {id} · {state.baseVersion || "MC"} · 模组 / 存档 / 资源`,
  `            {id}${state.baseVersion && state.baseVersion !== id ? ` · ${state.baseVersion}` : ""} · 模组 / 存档`
);
fs.writeFileSync("ui/src/pages/ContentPage.tsx", c);
console.log("ok");
