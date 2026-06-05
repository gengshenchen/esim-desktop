export const ERROR_CODES: Record<string, string> = {
  MODE: '未进入产测模式，请先执行 AT+PROD=1',
  UNSUP: '当前产品不支持该命令',
  ARG: '参数错误',
  FS: '文件系统错误',
  FMT: '配置格式错误（非空非注释行缺少 = 号）',
  IO: '文件/UART 读写错误',
  BUSY: '已有配置任务进行中',
  TO: 'MCU 超时未响应（检查 UART 连线）',
  FAIL: 'MCU 执行失败',
  DNS: 'DNS 解析失败',
  NOSIM: '未检测到 SIM 卡',
}

export function translateError(code: string): string {
  return ERROR_CODES[code] || `未知错误: ${code}`
}
