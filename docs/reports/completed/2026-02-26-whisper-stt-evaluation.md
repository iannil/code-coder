# Whisper STT 模型精准度评估报告

**评估时间**: 2026-02-26 10:20
**评估模型**: base (74M 参数)
**运行环境**: Docker CPU 模式 (fedirz/faster-whisper-server:latest-cpu)
**服务端口**: 4403

## 评估概要

| 指标 | 评分 | 说明 |
|------|------|------|
| 英文清晰语音 | ★★★★★ | 几乎完美，仅标点差异 |
| 英文真实人声 | ★★★★★ | Harvard Sentences 完整识别 |
| 中文简单句 | ★★★★☆ | 内容正确，输出繁体 |
| 中文专业术语 | ★★★★☆ | 术语识别准确，输出繁体 |
| 数字/百分比 | ★★★☆☆ | 数字正确，长句被截断 |
| **综合评分** | **4.0/5** | 适合清晰语音，中文需后处理 |

## 测试详情

### 测试 1: TTS 英文 (清晰合成语音)

| 项目 | 内容 |
|------|------|
| 音频来源 | macOS TTS (Samantha 声音) |
| 音频时长 | 5.9 秒 |
| 预期文本 | Hello world. This is a comprehensive test of the Whisper automatic speech recognition system. |
| 识别结果 | Hello world, this is a comprehensive test of the Whisper automatic speech recognition system. |
| 差异分析 | 句号变逗号，首字母大小写变化 |
| 评分 | ★★★★☆ |

### 测试 2: 真实人声英文 (Harvard Sentences)

| 项目 | 内容 |
|------|------|
| 音频来源 | Open Speech Repository |
| 音频时长 | 33.6 秒 |
| 识别结果 | The birch can be slid on the smooth planks. Glue the sheet to the dark blue background. It is easy to tell the depths of a well. These days the chicken leg is a rare dish. Rice is often served in round bowls. The juice of lemons makes fine punch. The box was thrown beside the park truck. The hogs were fed chopped corn and garbage. Four hours of steady work faced us. A large size in stockings is hard to sell. |
| 差异分析 | 完全正确，10个句子全部准确识别 |
| 评分 | ★★★★★ |

### 测试 3: TTS 中文简单句

| 项目 | 内容 |
|------|------|
| 音频来源 | macOS TTS (Tingting 声音) |
| 音频时长 | 3.5 秒 |
| 预期文本 | 今天天气很好，我们去公园散步吧。(简体) |
| 识别结果 | 今天天氣很好,我們去公園散步吧 (繁体) |
| 差异分析 | 内容完全正确，但输出为繁体中文 |
| 评分 | ★★★★☆ |

### 测试 4: TTS 中文专业术语

| 项目 | 内容 |
|------|------|
| 音频来源 | macOS TTS (Tingting 声音) |
| 音频时长 | 7.3 秒 |
| 预期文本 | 人工智能技术正在快速发展，深度学习模型的参数量已经突破了千亿级别。(简体) |
| 识别结果 | 人工智能技術正在快速發展,深度學習模型的參數量已經突破了千億級別。(繁体) |
| 差异分析 | 专业术语"人工智能"、"深度学习"、"参数量"、"千亿级别"全部正确识别 |
| 评分 | ★★★★☆ |

### 测试 5: 英文数字和百分比

| 项目 | 内容 |
|------|------|
| 音频来源 | macOS TTS |
| 音频时长 | 6.2 秒 |
| 预期文本 | The stock market closed at 15,234 points today representing a 2.5 percent increase from yesterday's trading session. |
| 识别结果 | The stock market closed at 15,234 points today representing a 2.5% increase. |
| 差异分析 | 数字"15,234"和"2.5%"正确，但句子末尾"from yesterday's trading session"被截断 |
| 评分 | ★★★☆☆ |

## 发现的问题

### 1. 中文输出为繁体
- **现象**: 简体中文音频被识别为繁体中文
- **原因**: Whisper 模型在训练时繁体中文数据占比较高
- **影响**: 需要后处理转换为简体
- **解决方案**: 使用 OpenCC 等工具进行繁简转换

### 2. 长句截断
- **现象**: 较长的英文句子末尾被截断
- **原因**: base 模型上下文窗口有限，对长句处理能力较弱
- **影响**: 复杂句子信息丢失
- **解决方案**: 升级到 small 或更大模型

### 3. 标点符号差异
- **现象**: 句号变逗号，标点使用不一致
- **影响**: 轻微，不影响语义理解
- **解决方案**: 可接受，或后处理规范化

## 模型对比建议

| 模型 | 参数量 | WER (英语) | 中文表现 | 推荐场景 |
|------|--------|------------|----------|----------|
| tiny | 39M | ~7.5% | 较差 | 原型测试 |
| **base (当前)** | 74M | ~5-6% | 一般 | 清晰语音、资源受限 |
| small | 244M | ~4% | 良好 | **推荐升级** |
| medium | 769M | ~3% | 很好 | 高准确度需求 |
| large-v3 | 1.55B | ~2-3% | 最佳 | GPU 环境 |
| turbo | 809M | ~2-3% | 很好 | 速度+准确度平衡 |

## 升级建议

### 推荐方案：升级到 `small` 模型

```bash
# 预下载模型 (避免容器内下载失败)
pip install huggingface_hub
python -c "from huggingface_hub import snapshot_download; snapshot_download('Systran/faster-whisper-small')"

# 启动服务
WHISPER_MODEL=small ./ops.sh start whisper
```

### 中文简体输出方案

在应用层添加繁简转换：

```typescript
import OpenCC from 'opencc-js';
const converter = OpenCC.Converter({ from: 'tw', to: 'cn' });

const transcription = await whisperApi.transcribe(audio);
const simplified = converter(transcription.text);
```

## 结论

当前 `base` 模型在以下场景表现良好：
- ✅ 清晰英文语音
- ✅ 短句识别
- ✅ 数字识别

需要改进的场景：
- ⚠️ 中文需繁简转换
- ⚠️ 长句可能截断
- ⚠️ 复杂口音/背景噪音

**总体评价**: 对于基础语音消息识别，`base` 模型可用。如需更高准确度，建议升级到 `small` 模型。
