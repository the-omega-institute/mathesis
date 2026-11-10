# Omega Institute 申请 Crossref 的 DOI 准备清单

本文按 Crossref 官方页面在 **2026-11-05** 的内容整理，目标是让 Omega
Institute 以后可以为 Mathesis 接受的论文、短文和研究报告注册自己的 DOI。
这里讨论的是 Mathesis 作为出版/研究内容生产者的 Crossref DOI；作者提交的
Zenodo、arXiv 或期刊 DOI 仍然保留为作者已有 DOI，二者不能互相替代。

## 结论

Omega Institute 作为研究内容生产组织，原则上符合 Crossref 的会员资格描述：
Crossref 对能产生专业或学术材料的组织开放会员，举例包括研究机构、出版者、
政府机构、研究资助者和博物馆。项目仓库中的 GitHub 组织名可以作为公开项目
身份，但申请必须使用能够签署 Crossref 会员条款并支付账单的**法律实体**；
仅有 `the-omega-institute` 的 GitHub 组织不能单独证明法律主体或付款主体。

优先比较两条路径：

1. **Independent member**：Omega Institute 直接签约、获得 Crossref 前缀，
   自己维护 landing page、元数据和注册程序。长期运营控制力最好，适合
   Mathesis 作为稳定出版平台使用。
2. **Sponsor member**：通过 Crossref sponsor 加入。Crossref 说明超过半数
   会员通过 sponsor 加入，通常不直接支付 Crossref 会员费并能获得本地技术
   支持；sponsor 可能另收服务费，需要比较服务范围、数据控制权、退出方式和
   DOI 前缀归属。若 Omega Institute 的法律、付款或技术条件尚未准备好，先
   用 sponsor 做小规模试发是合理的过渡方案。

申请入口：

- [Crossref membership / eligibility](https://www.crossref.org/membership/)
- [Independent member application](https://www.crossref.org/membership/_apply)
- [Working with a sponsor](https://www.crossref.org/membership/about-sponsors/)
- [Crossref fees](https://www.crossref.org/fees/)
- [Member setup documentation](https://www.crossref.org/documentation/member-setup/)
- [Constructing DOIs](https://www.crossref.org/documentation/member-setup/constructing-your-dois/)
- [Creating a landing page](https://www.crossref.org/documentation/member-setup/creating-a-landing-page/)
- [Crossref membership support ticket](https://support.crossref.org/hc/en-us/requests/new?ticket_form_id=360001618152)

## Omega Institute 申请前要准备的材料

由 Omega Institute 的授权负责人确认并留档：

- 法律实体全称、注册地和办公地址；官网和组织联系邮箱；授权签约人的姓名、
  职务和邮箱；账单联系人和可用的美元付款方式。
- 一段英文组织说明，说明 Omega Institute 运营 Mathesis，发布经审核的数学
  论文、短文、猜想或研究报告，并负责 DOI landing page、全文访问和元数据维护。
- Mathesis 的公开域名、单篇作品的稳定 URL 规则、隐私/版权/撤稿和长期维护
  联系方式。Crossref 要求每个 DOI 指向唯一且持续维护的 landing page。
- 第一批要注册的内容清单：标题、作者顺序、作者的 given/family name、
  ORCID（有则填）、机构、摘要、出版/在线日期、版本类型、版权/许可、全文
  URL、参考文献和已有 DOI（若有）。
- 对作者授权的记录：作者确认 Omega Institute 有权发布全文和登记公开书目
  元数据。Mathesis 现在的出版授权字段正是为此准备，且与 AI 处理授权分开。

申请时不要把 GitHub 仓库地址、Zenodo DOI 或 `WP-2026-0001` 当成法律主体、
Crossref 前缀或 DOI 本身。它们分别是项目身份、作者/外部存档标识和 Mathesis
本地记录号。

## 费用和决策依据

Crossref 的费用页说明，独立会员通常支付年度会员费和每条新 DOI/元数据记录的
一次性注册费；元数据后续更新不再收更新费，注册费通常按季度出账。2026 年
页面列出的出版内容年度会员费按出版收入或出版运营费用中较高者分档：低于
1,000 USD 为 200 USD；1,000–1,000,000 USD 为 275 USD；之后逐级增加。这个
数值会调整，且 Omega Institute 的实际档位必须以申请时法律实体的财务口径为准，
不能根据开源项目规模推断。

2026 年内容注册费中，与 Mathesis 最接近的类型是：

- technical reports / working papers 等当前记录每条 1.00 USD，back-year
  记录每条 0.15 USD；
- posted content（包括 preprints）当前记录每条 0.25 USD，back-year 每条
  0.15 USD。

Mathesis 最终应在 Crossref 申请或支持工单中确认论文、短文和报告应使用
`posted content` 还是 `reports and working papers`。这属于内容分类和会员
条款选择，不应由前端表单猜测。2027 年起 Crossref 已提示部分内容注册价格会
变化，因此上线预算要以实际注册年份的费用页为准。

## 获批后的技术流程

1. Crossref 分配一个 DOI prefix；Omega Institute 选择 suffix。Crossref 明确
   建议 suffix 使用不可解释、不可预测且不可变的 opaque 值。Mathesis 应把
   `WP-2026-0001` 留作本地 record id，生成一个独立的随机/不可预测 suffix，
   例如 `10.<prefix>/<opaque-id>`，一经注册永久绑定该版本内容。
2. 每个 DOI 解析到公开、稳定、唯一的论文 landing page。页面至少要有完整书目信息、
   以 `https://doi.org/...` 形式显示 DOI，以及全文访问方式；Crossref 不建议
   DOI 直接解析到会立即下载的 PDF。Mathesis 的公开论文页面应先具备这些条件，
   再提交注册。
3. 用 Web deposit / Metadata Manager 做小批量试发，用 HTTPS XML deposit 或
   受控的注册 worker 做生产注册。Crossref 会员凭据必须只在服务器 secret
   中使用，不能出现在浏览器或作者提交 JSON 中。
4. 注册前按不可变 publication snapshot 生成 Crossref metadata，保存元数据
   hash、agency、尝试次数、回执、注册时间和失败原因。成功后只写入
   `publication_doi`；作者原有 `doi` 永远不被覆盖。
5. 持续维护标题、作者、URL、许可、版本、撤稿/更正关系和参考文献。Crossref
   要求只登记自己有法律权利登记的内容，且 DOI 与 metadata 要在作品公开时及时
   创建；如果 landing page 迁移，必须更新 DOI 的 URL。

## 现有实现与申请条件的对应

本次改动已经把申请前最容易遗漏的字段放进提交契约：

- 作者可提交姓名、机构、ORCID、Google Scholar URL 和个人主页；输入式 conjecture
  与 LaTeX 上传共用同一作者字段。
- publication-purpose 必须勾选单独的出版与 DOI 元数据授权，并选择公开许可和
  可选 rights holder；private review 不会被要求授予出版授权。
- `Submission.doi` 明确表示作者已有 DOI；接受记录预留独立的
  `PublicationDoi` 状态和 `publication_doi` 公开字段，记录 Crossref/DataCite
  agency、pending/registered/failed、metadata hash、attempts、registered_at
  和 last_error。
- 这些授权的版本号、许可证和 rights holder 会随不可变 publication snapshot
  保存，后续注册 worker 可以从快照复现同一份 deposit。

仍需在实际申请前完成的组织工作是：确认 Omega Institute 的法律/付款主体，并确定
Mathesis 内容分类。公开 landing page、可抓取 HTML/PDF、sitemap/robots 的代码层
已经完成；部署后应按 [Google Scholar crawlability](GOOGLE_SCHOLAR.md) 中的匿名
`curl` 检查确认实际域名返回正确内容。Crossref XML deposit worker 和生产 secret
配置仍需在会员路径确定后完成。Crossref 会员申请、合同签署、付款和 DOI 正式注册
都需要 Omega Institute 授权负责人在外部页面完成，本仓库代码不会代替这些组织行为。

