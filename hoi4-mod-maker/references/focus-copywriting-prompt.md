# Focus Copywriting Prompt

Use this when Codex needs to write Chinese Hearts of Iron IV national focus titles and descriptions in the user's established style.

The style was inferred from the user's local `kxc` and `mdard` mods:

- `kxc`: 837 matched focus localisation entries, 802 with descriptions.
- `mdard`: 1117 matched focus localisation entries, 1031 with descriptions.
- Typical focus description length: about 80-120 Chinese characters.
- Major ideological, route-ending, or manifesto focuses can run 180-350 characters.
- Rare doctrine-heavy focuses may be longer, but should be used deliberately.

Do not copy existing text verbatim. Learn the cadence, framing, and political narrative style, then write new text for the requested focus.

## Core Style

Write like a Chinese HOI4 alternate-history mod, not like a product summary.

The voice is:

- political-historical,
- polemical but controlled,
- ideological rather than purely descriptive,
- written from inside the regime or movement's worldview,
- compact enough for a focus tooltip,
- confident, occasionally sardonic, but not internet-slang-heavy.

The usual paragraph logic is:

1. Start from a historical wound, institutional contradiction, class conflict, military crisis, or factional debate.
2. Interpret it through the regime's ideology or political line.
3. State why the new policy is necessary.
4. End with action, consolidation, rupture, or a new historical direction.

Good description skeleton:

```text
[Historical problem or contradiction]. [Ideological interpretation or factional judgment]. [Why the old path cannot continue]. [What the party/state/army/masses will now do].
```

Stronger route-ending skeleton:

```text
[A longer historical accounting]. [The opposing line or old order is named and judged]. [The new doctrine is declared]. [The final sentence turns the focus into a mandate, not a mere policy].
```

## Title Rules

Focus titles should be short and forceful. Prefer 2-10 Chinese characters unless a real doctrine name needs more.

Use:

- slogans: `大鸣大放`, `四个现代化`, `自古以来`,
- doctrine names: `新民主主义`, `苏维埃民主`, `马克思主义中国化`,
- policy phrases: `政企分开`, `取消官员特权`, `军队双轨制`,
- movement names: `批判儒家`, `土地改革`, `第二次文化革命`,
- person-route labels: `红色罗伊`, `李大钊主席`, `托派的脊梁`,
- sharp verb-object phrases: `肃清托洛茨基主义`, `警惕干部主义`, `控制共产党`.

Avoid:

- bland task names like `发展经济`, `加强军队`, `改善工业`,
- explaining effects in the title,
- overly modern marketing phrasing,
- titles longer than the UI can comfortably display.

## Description Length

Choose length by focus importance:

- Minor industry, army, research, or administrative focus: 60-110 Chinese characters.
- Normal political focus: 90-160 Chinese characters.
- Ideological branch focus: 140-260 Chinese characters.
- Route climax, chairman focus, constitutional focus, or civil-war settlement: 220-360 Chinese characters.
- Manifesto-style exceptional focus: 350-550 Chinese characters, only when explicitly requested.

If the user gives no preference, write 100-180 Chinese characters.

## Sentence Cadence

Use Chinese political prose with causal turns. Common sentence moves:

- `自...以来，...便...`
- `毫无疑问，...`
- `然而，...`
- `或许...，但...`
- `因此，...`
- `我们必须...`
- `这不是...，而是...`
- `只有...，才能...`
- `旧的...已经无法...`
- `革命并不会因为...而停止`
- `人民不会再容忍...`

The prose can use long sentences, but each sentence should push the argument forward.

## Vocabulary Bank

Political and class vocabulary:

- 党、国家、共和国、人民、群众、工人阶级、农民、干部、官僚、基层、先锋队
- 帝国主义、反动派、军阀、买办、寡头、资产阶级、封建残余、特权阶层
- 官僚主义、干部主义、形式民主、特供制度、地方主义、冒险主义、机会主义

Socialist and revolutionary vocabulary:

- 新民主主义、社会主义民主、苏维埃民主、人民民主专政、无产阶级专政
- 公社、国营经济、集体化、工业化、现代化、群众路线、继续革命
- 国际主义、一国建成社会主义、世界革命、民族解放、反帝统一战线

State-building vocabulary:

- 整顿、重组、改造、清算、巩固、统一、动员、普及、扩大、确立、恢复、推进
- 中央、地方、委员会、代表大会、安全机关、人民军队、国营企业、计划体系

Use these words as texture, not as a checklist.

## Tone Modes

Choose one tone mode based on the focus:

### Historical Policy

For industry, administration, diplomacy, army reform:

- controlled,
- sober,
- explains necessity,
- ends with a practical direction.

Example shape:

```text
长期的割据与战争使旧有制度只剩下空壳。若要让国家重新运转，我们必须把分散的权力、资源与责任重新纳入统一的计划之中。新的机构不会只是纸面上的改革，而将成为共和国继续前进的骨架。
```

### Ideological Debate

For faction choices, doctrine focuses, party congresses:

- names the opposing line,
- judges it through class or party logic,
- declares the chosen line as historically necessary.

Example shape:

```text
党内的争论从来不是单纯的理论游戏，而是国家未来道路的预演。保守的妥协会使革命停留在昨日，盲目的冒进又会把人民推向新的灾难。我们必须在斗争中确立一条能够组织群众、改造社会并保卫共和国的路线。
```

### Revolutionary Mobilisation

For war, uprising, liberation, anti-imperialism:

- energetic,
- collective,
- uses masses/army/front language,
- ends with action.

Example shape:

```text
敌人仍然相信旧秩序能够靠枪炮与金钱苟延残喘，但他们已经忘记了人民为何拿起武器。工人、农民与士兵将被重新组织起来，前线不再只是军队的前线，而是整个革命的前线。
```

### Satirical Or Strange Route

For absurd, dark, or experimental paths:

- still written as in-universe policy,
- can be ironic,
- should not collapse into pure meme.

Example shape:

```text
旁观者也许会把这一计划当作疯人的幻想，但他们从未真正理解旧社会本身有多么荒诞。既然所谓常识只是旧秩序的遮羞布，那么我们便要把这块布彻底扯下，让新的原则以最直接的方式统治现实。
```

## Output Format

When asked for localisation, output this shape:

```yaml
l_simp_chinese:
  TAG_focus_id:0 "标题"
  TAG_focus_id_desc:0 "描述"
```

When asked for a focus-writing batch, output this shape:

```yaml
focus_copy:
  - id: TAG_focus_id
    title: 标题
    desc: 描述
    tone: historical_policy | ideological_debate | revolutionary_mobilisation | strange_route
```

Descriptions should be one localisation-safe string. Escape internal quotes if writing final `.yml`.

## Prompt To Use

```text
你是钢铁雄心4中文国策文案作者。请按我的本地 mod 文案风格，为下面的国策写中文标题与描述。

风格要求：
- 写成 HOI4 架空历史国策文案，不要写成现代说明书。
- 标题短促有力，像政策名、政治口号、路线名、运动名或人物路线标签。
- 描述采用“历史矛盾/现实困境 -> 阶级或制度解释 -> 政策必要性 -> 行动或历史方向”的结构。
- 语气要像政治史论、路线斗争、革命宣言或国家建设文件，允许有讽刺，但不要网文腔。
- 必须以本国、本路线或本利益集团的内部第一视角写；可以使用“我们”、党、政府、军队、共和国等自我称谓。
- 不要直接列游戏效果，不要说“该国策将给予...”，除非用户明确要求机制说明。
- 不要抄已有文案，保持同类节奏与词汇质感即可。
- 默认描述 100-180 字；如果是路线终点、党代会、宪法、主席/领袖国策，可写 220-360 字。

交付硬规则：
- 不准使用“先做可校验 demo”“保守脚本骨架”“之后补回文案/路线叙事”作为交付策略或自我解释。
- 不准把可编译、可校验的骨架当成完成品；生成前必须先抽取路线叙事，最终输出必须包含完成态标题、描述、本地化和脚本。
- 不准输出第三方视角、百科视角、历史学者旁白或“他们/该国/该政权将...”式外部评价。
- 不准在 `l_simp_chinese:` 下生成 `<prefix>_mod_name`、`chinaprc_1979_mod_name` 或任何 `*_mod_name`；mod 名称只写在 `descriptor.mod` 和外层 `.mod` 文件。

输入：
国家/势力：{国家或势力}
时间线背景：{世界线背景}
所属路线：{政治路线或分支}
国策ID：{focus_id}
国策作用：{这个国策在剧情/机制上的作用}
前置矛盾：{上一阶段的问题或争论}
希望语气：{historical_policy / ideological_debate / revolutionary_mobilisation / strange_route}
关键词：{必须出现或可参考的词}
长度：{短/中/长，可省略}

输出：
1. 标题
2. 描述
3. 若需要写入本地化，给出：
   {focus_id}:0 "标题"
   {focus_id}_desc:0 "描述"
```

## Quality Checklist

Before finalising, check:

- The title fits a focus icon tooltip and is not a full sentence.
- The description does not mention raw game effects.
- The first sentence creates historical or political pressure.
- The middle sentence explains why the chosen line is necessary.
- The last sentence gives direction, not just summary.
- The text sounds like an in-universe faction justifying itself.
- If the focus is minor, the description is not bloated.
- If the focus is a route climax, it has enough ideological weight.
- Localisation output keeps quotes escaped and stays on one line.
