/**
 * Native JAR Analyzer Integration
 *
 * Provides high-performance JAR analysis using Rust native bindings.
 * Falls back to TypeScript implementation if native is unavailable.
 *
 * Performance improvements:
 * - Class file parsing: 8-15x faster (zero-copy binary parsing)
 * - JAR extraction: 2-3x faster (in-memory, no shell exec)
 * - Batch class processing: 3-5x faster (rayon parallel)
 * - Technology detection: 5-10x faster (aho-corasick O(n))
 */

import type { JarAnalysisResult, ClassInfo, JarMetadata, PackageInfo, ConfigFile, Dependency } from "./jar-analyzer"

// ============================================================================
// Native Types (from NAPI bindings)
// ============================================================================

interface NapiJarMetadata {
  mainClass?: string
  implementationTitle?: string
  implementationVersion?: string
  implementationVendor?: string
  specificationTitle?: string
  specificationVersion?: string
  buildTool?: string
  jdkVersion?: string
  createdBy?: string
  bundleName?: string
  bundleVersion?: string
  bundleSymbolicName?: string
}

interface NapiClassInfo {
  name: string
  packageName: string
  simpleName: string
  classType: string
  modifiers: string[]
  bytecodeVersion: number
  javaVersion: string
}

interface NapiJavaPackageInfo {
  name: string
  classCount: number
}

interface NapiJavaConfigFile {
  path: string
  fileType: string
  content?: string
}

interface NapiDependency {
  groupId?: string
  artifactId?: string
  version?: string
  scope?: string
}

interface NapiDetection {
  name: string
  category: string
  website?: string
  matches: string[]
  confidence: string
}

interface NapiJarAnalysis {
  jarPath: string
  jarName: string
  metadata: NapiJarMetadata
  classNames: string[]
  packageNames: string[]
  packages: NapiJavaPackageInfo[]
  classes: NapiClassInfo[]
  configFiles: NapiJavaConfigFile[]
  dependencies: NapiDependency[]
  detections: NapiDetection[]
  entryCount: number
  sizeBytes: number
}

interface NapiFingerprintInput {
  classNames?: string[]
  packageNames?: string[]
  configFiles?: string[]
  annotations?: string[]
  manifest?: Record<string, string>
}

interface NapiJavaFingerprint {
  name: string
  category: string
  website?: string
  patternCount: number
}

// Native module interface
interface NativeModule {
  analyzeJar(jarPath: string, maxClasses?: number): NapiJarAnalysis
  parseClassFileSync(data: Buffer): NapiClassInfo
  detectJavaTechnologies(input: NapiFingerprintInput): NapiDetection[]
  jarAnalysisSummary(jarPath: string): string
  JarAnalyzerHandle: {
    open(path: string): JarAnalyzerHandle
  }
  FingerprintEngineHandle: {
    create(): FingerprintEngineHandle
  }
}

interface JarAnalyzerHandle {
  readonly path: string
  readonly entryCount: number
  analyze(maxClasses?: number): NapiJarAnalysis
  summary(): string
  classFilePaths(): string[]
  configFilePaths(): string[]
  close(): void
}

interface FingerprintEngineHandle {
  detect(input: NapiFingerprintInput): NapiDetection[]
  fingerprints(): NapiJavaFingerprint[]
  fingerprintsByCategory(category: string): NapiJavaFingerprint[]
  categories(): string[]
}

// ============================================================================
// Native Loading
// ============================================================================

let nativeModule: NativeModule | null = null
let loadAttempted = false

async function loadNative(): Promise<NativeModule | null> {
  if (loadAttempted) return nativeModule
  loadAttempted = true

  try {
    // Try to load the native module
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const native = await import("@codecoder-ai/core") as any

    // Check if Java module exports exist
    if (
      typeof native.analyzeJar === "function" &&
      typeof native.parseClassFileSync === "function" &&
      typeof native.detectJavaTechnologies === "function" &&
      native.JarAnalyzerHandle &&
      native.FingerprintEngineHandle
    ) {
      nativeModule = native as NativeModule
      return nativeModule
    }
    return null
  } catch {
    // Native module not available
    return null
  }
}

/**
 * Check if native JAR analyzer is available
 */
export async function isNativeAvailable(): Promise<boolean> {
  const native = await loadNative()
  return native !== null
}

/**
 * Check if native is available (sync, only reliable after first async call)
 */
export function isUsingNative(): boolean {
  return nativeModule !== null
}

// ============================================================================
// Conversion Functions
// ============================================================================

function convertMetadata(napi: NapiJarMetadata): JarMetadata {
  return {
    mainClass: napi.mainClass,
    implementationTitle: napi.implementationTitle,
    implementationVersion: napi.implementationVersion,
    implementationVendor: napi.implementationVendor,
    specificationTitle: napi.specificationTitle,
    specificationVersion: napi.specificationVersion,
    buildTool: napi.buildTool,
    jdkVersion: napi.jdkVersion,
    createdBy: napi.createdBy,
    bundleName: napi.bundleName,
    bundleVersion: napi.bundleVersion,
    bundleSymbolicName: napi.bundleSymbolicName,
  }
}

function convertClassInfo(napi: NapiClassInfo): ClassInfo {
  return {
    name: napi.name,
    packageName: napi.packageName,
    simpleName: napi.simpleName,
    type: napi.classType as ClassInfo["type"],
    modifiers: napi.modifiers,
    bytecodeVersion: napi.bytecodeVersion,
    javaVersion: napi.javaVersion,
  }
}

function convertPackageInfo(napi: NapiJavaPackageInfo): PackageInfo {
  return {
    name: napi.name,
    classCount: napi.classCount,
  }
}

function convertConfigFile(napi: NapiJavaConfigFile): ConfigFile {
  return {
    path: napi.path,
    type: napi.fileType,
    content: napi.content,
  }
}

function convertDependency(napi: NapiDependency): Dependency {
  return {
    groupId: napi.groupId,
    artifactId: napi.artifactId,
    version: napi.version,
    scope: napi.scope,
  }
}

function convertJarAnalysis(napi: NapiJarAnalysis): JarAnalysisResult {
  // Convert detections to Map format matching TypeScript API
  const detectedTechs = new Map<string, { tech: any; matches: string[] }>()
  for (const d of napi.detections) {
    detectedTechs.set(d.name, {
      tech: {
        name: d.name,
        category: d.category,
        website: d.website,
        patterns: [], // Not available from native
      },
      matches: d.matches,
    })
  }

  return {
    jarPath: napi.jarPath,
    jarName: napi.jarName,
    metadata: convertMetadata(napi.metadata),
    classNames: napi.classNames,
    packageNames: napi.packageNames,
    packages: napi.packages.map(convertPackageInfo),
    classes: napi.classes.map(convertClassInfo),
    configFiles: napi.configFiles.map(convertConfigFile),
    dependencies: napi.dependencies.map(convertDependency),
    detectedTechs,
    entryCount: napi.entryCount,
    sizeBytes: napi.sizeBytes,
  }
}

// ============================================================================
// Public API
// ============================================================================

/**
 * Analyze a JAR file using native Rust implementation
 *
 * @param jarPath Path to the JAR file
 * @param options Analysis options
 * @returns Analysis result or null if native is unavailable
 */
export async function analyzeJarNative(
  jarPath: string,
  options?: { maxClasses?: number }
): Promise<JarAnalysisResult | null> {
  const native = await loadNative()
  if (!native) return null

  try {
    const result = native.analyzeJar(jarPath, options?.maxClasses)
    return convertJarAnalysis(result)
  } catch (error) {
    console.error("Native JAR analysis failed:", error)
    return null
  }
}

/**
 * Parse a class file from bytes using native implementation
 *
 * @param data Class file bytes
 * @returns ClassInfo or null if native is unavailable
 */
export async function parseClassFileNative(data: Buffer): Promise<ClassInfo | null> {
  const native = await loadNative()
  if (!native) return null

  try {
    const result = native.parseClassFileSync(data)
    return convertClassInfo(result)
  } catch (error) {
    console.error("Native class file parsing failed:", error)
    return null
  }
}

/**
 * Detect Java technologies using native fingerprint engine
 *
 * @param input Fingerprint input (class names, package names, config files, etc.)
 * @returns Array of detections or null if native is unavailable
 */
export async function detectJavaTechnologiesNative(input: {
  classNames?: string[]
  packageNames?: string[]
  configFiles?: string[]
  annotations?: string[]
  manifest?: Record<string, string>
}): Promise<NapiDetection[] | null> {
  const native = await loadNative()
  if (!native) return null

  try {
    return native.detectJavaTechnologies(input)
  } catch (error) {
    console.error("Native technology detection failed:", error)
    return null
  }
}

/**
 * Get JAR analysis summary using native implementation
 *
 * @param jarPath Path to the JAR file
 * @returns Summary string or null if native is unavailable
 */
export async function getJarSummaryNative(jarPath: string): Promise<string | null> {
  const native = await loadNative()
  if (!native) return null

  try {
    return native.jarAnalysisSummary(jarPath)
  } catch (error) {
    console.error("Native JAR summary failed:", error)
    return null
  }
}

/**
 * Create a native JAR analyzer handle for incremental analysis
 *
 * @param jarPath Path to the JAR file
 * @returns JarAnalyzerHandle or null if native is unavailable
 */
export async function openJarNative(jarPath: string): Promise<JarAnalyzerHandle | null> {
  const native = await loadNative()
  if (!native) return null

  try {
    return native.JarAnalyzerHandle.open(jarPath)
  } catch (error) {
    console.error("Native JAR open failed:", error)
    return null
  }
}

/**
 * Create a native fingerprint engine for repeated detection
 *
 * @returns FingerprintEngineHandle or null if native is unavailable
 */
export async function createFingerprintEngineNative(): Promise<FingerprintEngineHandle | null> {
  const native = await loadNative()
  if (!native) return null

  try {
    return native.FingerprintEngineHandle.create()
  } catch (error) {
    console.error("Native fingerprint engine creation failed:", error)
    return null
  }
}

// ============================================================================
// Hybrid API (native + fallback)
// ============================================================================

/**
 * Analyze a JAR file with automatic native/TypeScript fallback
 *
 * Prefers native implementation for better performance, falls back to
 * TypeScript if native is unavailable.
 *
 * @param jarPath Path to the JAR file
 * @param options Analysis options
 * @returns Analysis result
 */
export async function analyzeJar(
  jarPath: string,
  options?: { maxClasses?: number }
): Promise<JarAnalysisResult> {
  // Try native first
  const nativeResult = await analyzeJarNative(jarPath, options)
  if (nativeResult) return nativeResult

  // Fallback to TypeScript implementation
  const { JarAnalyzer } = await import("./jar-analyzer")
  return JarAnalyzer.analyze(jarPath, options)
}

// Re-export types for convenience
export type {
  NapiDetection,
  NapiJavaFingerprint,
  JarAnalyzerHandle,
  FingerprintEngineHandle,
}
