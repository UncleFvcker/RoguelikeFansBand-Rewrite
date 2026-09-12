# SPDX-License-Identifier: MPL-2.0
param(
  [Parameter(Mandatory = $true)][string]$Executable,
  [Parameter(Mandatory = $true)][string]$OutputDirectory,
  [ValidateSet('Mage', 'Ranger', 'Priest', 'WarriorMage', 'MagicEater')][string]$Class = 'Mage'
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
$executablePath = (Resolve-Path -LiteralPath $Executable).Path
$outputPath = [IO.Path]::GetFullPath($OutputDirectory)
[IO.Directory]::CreateDirectory($outputPath) | Out-Null
$previousProfile = $env:WEBVIEW2_USER_DATA_FOLDER
$env:WEBVIEW2_USER_DATA_FOLDER = Join-Path $outputPath 'webview-profile'
$app = Start-Process -FilePath $executablePath -WindowStyle Hidden -PassThru
try {
  $deadline = [DateTime]::UtcNow.AddSeconds(30)
  do { Start-Sleep -Milliseconds 100; $app.Refresh() } while ($app.MainWindowHandle -eq 0 -and [DateTime]::UtcNow -lt $deadline)
  if ($app.MainWindowHandle -eq 0) { throw 'Native window did not open' }
  $root = [System.Windows.Automation.AutomationElement]::FromHandle($app.MainWindowHandle)
  function FindElement($property, [string]$value) {
    $condition = New-Object System.Windows.Automation.PropertyCondition($property, $value)
    $deadline = [DateTime]::UtcNow.AddSeconds(30)
    do {
      $element = $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $condition)
      if ($element -and $element.Current.IsEnabled) { return $element }
      Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Native control missing: $value"
  }
  function ById([string]$id) { FindElement ([System.Windows.Automation.AutomationElement]::AutomationIdProperty) $id }
  function ByName([string]$name) { FindElement ([System.Windows.Automation.AutomationElement]::NameProperty) $name }
  function Invoke($element) { Write-Output "Invoking $($element.Current.Name)"; $element.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke() }
  $className = switch ($Class) { 'Ranger' { '游侠' }; 'Priest' { '牧师' }; 'WarriorMage' { '战法师' }; 'MagicEater' { '食魔者' }; default { '法师' } }
  $classGroup = switch ($Class) { 'Ranger' { '箭术' }; 'Priest' { '祈祷' }; 'WarriorMage' { '混合' }; 'MagicEater' { '魔法装置' }; default { '魔法' } }
  $capacity = if ($Class -eq 'Ranger') { 0 } else { 1 }
  # The raw HTML button exists before localization and session handlers are ready.
  Invoke (ByName '新游戏')
  (ById 'session-character-name').GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).SetValue("$Class Smoke")
  (ById 'session-seed').GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).SetValue('925')
  (ById 'session-tab-career').GetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern).Select()
  (ByName $classGroup).GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern).Toggle()
  if ($Class -eq 'MagicEater') {
    (ByName $className).GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern).Toggle()
  } else {
    Invoke (ByName "$className，进入领域选择")
    if ($Class -eq 'Mage') { Invoke (ByName '奥秘，进入领域选择') }
    if ($Class -eq 'Priest') { Invoke (ByName '生命，进入领域选择') }
    (ByName '咒术').GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern).Toggle()
  }
  $start = ById 'session-start-game'
  if (-not $start.Current.IsEnabled) { throw "$Class realm selection did not enable creation" }
  Invoke $start
  $identity = (ByName "$Class Smoke 人类$className").Current.Name
  $level = (ByName '1 / 1').Current.Name
  $budget = if ($Class -eq 'MagicEater') { 'No book caster' } else { (ByName "已学 0 / $capacity · 剩余 $capacity 个容量").Current.Name }
  $location = (ByName '前哨站').Current.Name
  $resourceList = if ($Class -eq 'MagicEater') {
    $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, (New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::AutomationIdProperty, 'resource-list')))
  } else { ById 'resource-list' }
  $resourceNames = if ($resourceList) { @($resourceList.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition) | ForEach-Object { $_.Current.Name }) } else { @() }
  if ($Class -eq 'MagicEater') {
    if ($resourceNames -match '法力') { throw 'Magic-Eater must not expose a shared mana pool' }
    (ByName '菜单').GetCurrentPattern([System.Windows.Automation.ExpandCollapsePattern]::Pattern).Expand()
    Invoke (ById 'player-ui-ability-open')
    Invoke (ById 'magic-eater-open')
    ById 'magic-eater-slots' | Out-Null
  } elseif (-not ($resourceNames -match '法力')) { throw "$Class mana was not rendered" }
  if (-not $app.CloseMainWindow()) { throw 'Native window did not accept close' }
  if (-not $app.WaitForExit(10000)) { throw 'Native process did not exit normally' }
  if ($app.ExitCode -ne 0) { throw "Native process exited with $($app.ExitCode)" }
  [ordered]@{
    passed = $true
    executable = $executablePath
    executableSha256 = (Get-FileHash -LiteralPath $executablePath -Algorithm SHA256).Hash.ToLowerInvariant()
    checkedAtUtc = [DateTime]::UtcNow.ToString('o')
    method = 'Optimized standalone EXE; Windows UI Automation Invoke/Toggle/SelectionItem/Value patterns; no WebDriver or test commands'
    checks = if ($Class -eq 'MagicEater') { @('title', 'normal human Magic-Eater creation without realms', 'level 1 without shared mana', 'body-device menu opens', 'normal process exit') } else { @('title', "normal human $Class creation with Sorcery secondary realm", 'level 1, mana and shared learning capacity', 'normal process exit') }
    identity = $identity; level = $level; learningBudget = $budget; location = $location; resources = $resourceNames
    preparation = 'Fresh WebView profile, seed 925, normal creation only; no granted XP, books or devices'
    limitations = 'Native smoke covers creation and initial projection only. Gameplay, save determinism and screenshots are recorded separately by the same-source WebDriver build.'
    exitCode = $app.ExitCode
  } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $outputPath 'checks.json') -Encoding UTF8
  Write-Output "Optimized $Class native creation smoke passed."
} finally {
  $env:WEBVIEW2_USER_DATA_FOLDER = $previousProfile
  if (-not $app.HasExited) { $app.CloseMainWindow() | Out-Null; if (-not $app.WaitForExit(5000)) { Stop-Process -Id $app.Id } }
}
