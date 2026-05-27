# 规则引擎

在 `config.yaml` 中声明规则，每条 MQTT 消息自动通过规则引擎评估。

## 基本结构

```yaml
rules:
  - name: "高温告警"
    device: "pump/*"         # 设备匹配（支持 * 通配符）
    metric: "temperature"    # 监听的指标名
    condition: "value > 80"  # 触发条件
    action: "alert"
    ai_enhance: true         # 触发后自动 LLM 分析
```

## 三种条件语法

### 数值阈值

```yaml
condition: "value > 80"    # 大于 80
condition: "value < 10"    # 小于 10
condition: "value >= 100"  # 大于等于
```

### 变化速率

```yaml
condition: "rate > 5"      # 每分钟变化超过 5 个单位
```

基于滑动窗口计算。比如温度从 80 飙到 90，每分钟 +10，rate = 10，触发。

### 设备离线

```yaml
condition: "last_seen > 300s"  # 5 分钟无数据
condition: "last_seen > 60s"   # 60 秒无数据
```

## 设备匹配

```yaml
device: "*"           # 所有设备
device: "pump/*"      # pump/1、pump/2...
device: "pump/3"      # 精确匹配
```

## 严重程度自动分级

规则名含"温度"或"高温"时，自动按阈值分级：

| 温度 | 级别 |
|------|------|
| 80–88°C | info |
| 88–100°C | warning |
| 100°C+ | critical |

规则名含 "critical" / "严重" → 直接 critical。

## 完整示例

```yaml
rules:
  - name: "泵高温告警"
    device: "pump/*"
    metric: "temperature"
    condition: "value > 80"
    action: "alert"
    ai_enhance: true

  - name: "温度飙升"
    device: "*"
    metric: "temperature"
    condition: "rate > 8"
    action: "alert"
    ai_enhance: true

  - name: "设备离线"
    device: "*"
    metric: "status"
    condition: "last_seen > 300s"
    action: "alert"
    ai_enhance: false
```
