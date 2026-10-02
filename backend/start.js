#!/usr/bin/env node

/**
 * SSH Assistant - .NET Aspire 启动脚本
 *
 * 用于启动和管理 .NET Aspire 分布式应用
 *
 * 使用方法:
 *   node start.js              - 启动 Aspire AppHost（包含 API + 前端）
 *   node start.js --api        - 仅启动 Admin API（不启动前端）
 *   node start.js --build      - 构建后启动
 *   node start.js --watch      - 监视文件变化并自动重启
 *   node start.js --prod       - 生产模式启动
 */

const { spawn, exec } = require('child_process');
const path = require('path');
const fs = require('fs');

// 配置
const CONFIG = {
  // Aspire AppHost 项目路径
  appHostPath: path.join(__dirname, 'SshAssistant.AppHost'),
  // 单独启动 API 时的项目路径（备用）
  adminApiPath: path.join(__dirname, 'SshAssistant.AdminApi'),
  // 默认端口（用于检查）
  ports: [5000, 5001, 5173, 8000],
  logPrefix: '[Aspire]'
};

// 颜色输出
const colors = {
  reset: '\x1b[0m',
  red: '\x1b[31m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  cyan: '\x1b[36m'
};

function log(message, color = 'reset') {
  console.log(`${colors[color]}${CONFIG.logPrefix} ${message}${colors.reset}`);
}

function error(message) {
  log(message, 'red');
}

function success(message) {
  log(message, 'green');
}

function info(message) {
  log(message, 'cyan');
}

// 检查端口是否被占用
function checkPort(port) {
  return new Promise((resolve) => {
    const net = require('net');
    const server = net.createServer();

    server.once('error', () => {
      resolve(false); // 端口被占用
    });

    server.once('listening', () => {
      server.close();
      resolve(true); // 端口可用
    });

    server.listen(port);
  });
}

// 检查多个端口
async function checkPorts(ports) {
  const results = [];
  for (const port of ports) {
    const available = await checkPort(port);
    results.push({ port, available });
  }
  return results;
}

// 检查 .NET 是否安装
function checkDotNet() {
  return new Promise((resolve) => {
    exec('dotnet --version', (error, stdout) => {
      if (error) {
        error('错误: 未检测到 .NET SDK，请先安装 .NET');
        resolve(false);
      } else {
        success(`检测到 .NET SDK: ${stdout.trim()}`);
        resolve(true);
      }
    });
  });
}

// 启动 Aspire AppHost
function startAspire(args = []) {
  const dotnetArgs = ['run', '--project', CONFIG.appHostPath, ...args];

  info(`启动 Aspire AppHost: dotnet ${dotnetArgs.join(' ')}`);
  console.log('  ├─ admin-api (.NET Web API)');
  console.log('  └─ admin-web (Vite 前端)');
  console.log('');

  const child = spawn('dotnet', dotnetArgs, {
    stdio: 'inherit',
    shell: true,
    env: { ...process.env, ASPNETCORE_ENVIRONMENT: 'Development' }
  });

  child.on('error', (err) => {
    error(`启动失败: ${err.message}`);
  });

  child.on('exit', (code) => {
    if (code !== 0) {
      error(`进程退出，代码: ${code}`);
    }
  });

  return child;
}

// 仅启动 API（备用选项）
function startAdminApi(args = []) {
  const dotnetArgs = ['run', '--project', CONFIG.adminApiPath, ...args];

  info(`启动 Admin API: dotnet ${dotnetArgs.join(' ')}`);

  const child = spawn('dotnet', dotnetArgs, {
    stdio: 'inherit',
    shell: true,
    env: { ...process.env, ASPNETCORE_ENVIRONMENT: 'Development' }
  });

  child.on('error', (err) => {
    error(`启动失败: ${err.message}`);
  });

  child.on('exit', (code) => {
    if (code !== 0) {
      error(`进程退出，代码: ${code}`);
    }
  });

  return child;
}

// 监视模式（需要 dotnet-watch 或手动重启）
function watchMode() {
  info('监视模式: 检测到文件变化时将重启服务器...');

  const apiOnly = process.argv.includes('--api');
  let currentProcess = null;

  function restart() {
    if (currentProcess) {
      currentProcess.kill();
    }
    info('重启服务器...');
    currentProcess = apiOnly ? startAdminApi() : startAspire();
  }

  // 简单的文件监视
  const watchPath = apiOnly
    ? path.join(CONFIG.adminApiPath, '**/*.cs')
    : path.join(CONFIG.appHostPath, '**/*.cs');

  // 使用 chokidar 或简单的轮询（这里用简单的实现）
  info('提示: 安装 dotnet-watch 获得更好的监视体验');
  info('运行: dotnet tool install -g dotnet-watch');

  restart();

  // 这里可以添加文件监视逻辑
  // 或者使用 dotnet watch run
}

// 主函数
async function main() {
  const args = process.argv.slice(2);

  console.log('\n' + '='.repeat(50));
  info('SSH Assistant - .NET Aspire 启动器');
  console.log('='.repeat(50) + '\n');

  // 检查 .NET
  const hasDotNet = await checkDotNet();
  if (!hasDotNet) {
    process.exit(1);
  }

  // 解析参数
  const apiOnly = args.includes('--api');
  const shouldBuild = args.includes('--build');
  const shouldWatch = args.includes('--watch');
  const isProd = args.includes('--prod');

  // 选择启动模式
  const startMode = apiOnly ? 'Admin API only' : 'Aspire AppHost (API + Frontend)';
  info(`启动模式: ${startMode}`);

  // 检查项目目录
  const projectPath = apiOnly ? CONFIG.adminApiPath : CONFIG.appHostPath;
  if (!fs.existsSync(projectPath)) {
    error(`错误: 项目目录不存在: ${projectPath}`);
    process.exit(1);
  }

  // 检查端口
  const portResults = await checkPorts(CONFIG.ports);
  const occupiedPorts = portResults.filter(r => !r.available).map(r => r.port);

  if (occupiedPorts.length > 0) {
    error(`警告: 以下端口可能已被占用: ${occupiedPorts.join(', ')}`);
  }

  // 构建处理
  if (shouldBuild) {
    info('构建模式: 先构建项目...');
    await new Promise((resolve, reject) => {
      exec('dotnet build', { cwd: projectPath }, (error) => {
        if (error) {
          error('构建失败');
          reject(error);
        } else {
          success('构建成功');
          resolve();
        }
      });
    });
  }

  // 环境设置
  if (isProd) {
    info('生产模式: 设置环境变量为 Production');
    process.env.ASPNETCORE_ENVIRONMENT = 'Production';
  } else {
    info('开发模式: 设置环境变量为 Development');
  }

  console.log('');

  // 启动进程
  if (shouldWatch) {
    watchMode();
  } else {
    const process = apiOnly ? startAdminApi() : startAspire();

    // 处理 Ctrl+C
    process.on('SIGINT', () => {
      console.log('\n');
      info('收到退出信号，正在关闭...');
      process.kill();
      process.exit(0);
    });
  }
}

// 启动
main().catch(err => {
  error(`启动失败: ${err.message}`);
  process.exit(1);
});
