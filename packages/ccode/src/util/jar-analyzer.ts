/**
 * JAR Analyzer
 *
 * Analyzes JAR files to extract metadata, class structure, and detect technologies.
 * Uses Bun's built-in unzip capabilities and file system operations.
 */

import { mkdir, mkdtemp, rm } from "node:fs/promises"
import path from "path"
import { findJavaFingerprints } from "./java-fingerprints"

async function exists(filePath: string): Promise<boolean> {
  try {
    await Bun.file(filePath).exists()
    return true
  } catch {
    return false
  }
}

// ============================================================================
// Types
// ============================================================================

export interface JarMetadata {
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

export interface ClassInfo {
  name: string
  packageName: string
  simpleName: string
  type: "class" | "interface" | "enum" | "annotation"
  modifiers: string[]
  bytecodeVersion?: number
  javaVersion?: string
}

export interface PackageInfo {
  name: string
  classCount: number
}

export interface ConfigFile {
  path: string
  type: string
  content?: string
}

export interface Dependency {
  groupId?: string
  artifactId?: string
  version?: string
  scope?: string
}

export interface JarAnalysisResult {
  jarPath: string
  jarName: string
  metadata: JarMetadata
  classNames: string[]
  packageNames: string[]
  packages: PackageInfo[]
  classes: ClassInfo[]
  configFiles: ConfigFile[]
  dependencies: Dependency[]
  detectedTechs: Map<string, { tech: any; matches: string[] }>
  entryCount: number
  sizeBytes: number
}

// ============================================================================
// Constants
// ============================================================================

const MANIFEST_PATH = "META-INF/MANIFEST.MF"
const POM_PROPERTIES_PATH = "META-INF/maven/**/*.pom.properties"
const POM_XML_PATH = "META-INF/maven/**/pom.xml"

const CONFIG_FILE_PATTERNS = [
  "*.properties",
  "*.yml",
  "*.yaml",
  "*.xml",
  "spring*.xml",
  "hibernate*.xml",
  "web.xml",
  "persistence.xml",
  "ejb-jar.xml",
  "faces-config.xml",
  "struts.xml",
  "applicationContext.xml",
  "log4j2.xml",
  "log4j.properties",
  "logback.xml",
]

const CLASS_FILE_EXT = ".class"

const BYTECODE_VERSION_MAP: Record<number, string> = {
  45: "1.1",
  46: "1.2",
  47: "1.3",
  48: "1.4",
  49: "5.0",
  50: "6.0",
  51: "7.0",
  52: "8.0",
  53: "9.0",
  54: "10.0",
  55: "11.0",
  56: "12.0",
  57: "13.0",
  58: "14.0",
  59: "15.0",
  60: "16.0",
  61: "17.0",
  62: "18.0",
  63: "19.0",
  64: "20.0",
  65: "21.0",
  66: "22.0",
  67: "23.0",
  68: "24.0",
}

// ============================================================================
// JAR Analyzer Class
// ============================================================================

export namespace JarAnalyzer {
  /**
   * Analyze a JAR file and extract all relevant information
   */
  export async function analyze(jarPath: string, options: { maxClasses?: number } = {}): Promise<JarAnalysisResult> {
    const maxClasses = options.maxClasses ?? 5000

    // Validate JAR exists
    if (!(await exists(jarPath))) {
      throw new Error(`JAR file not found: ${jarPath}`)
    }

    const jarName = path.basename(jarPath)
    const jarFile = Bun.file(jarPath)
    const fileExists = await jarFile.exists()
    const stat = fileExists ? jarFile.size : 0

    // Create temp directory for extraction
    const tempDir = await mkdtemp(path.join(path.dirname(jarPath), ".jar-analyze-"))

    try {
      // Extract JAR using bun's unzip
      await extractJar(jarPath, tempDir)

      // Analyze components
      const metadata = await extractMetadata(tempDir)
      const classes = await extractClasses(tempDir, maxClasses)
      const configFiles = await extractConfigFiles(tempDir)
      const dependencies = await extractDependencies(tempDir)
      const classNames = classes.map((c) => c.name)
      const packageNames = Array.from(new Set(classes.map((c) => c.packageName)))

      // Detect technologies
      const detectedTechs = findJavaFingerprints({
        classNames,
        packageNames,
        configFiles: configFiles.map((f) => f.path),
      })

      // Build package hierarchy
      const packageMap = new Map<string, number>()
      for (const cls of classes) {
        packageMap.set(cls.packageName, (packageMap.get(cls.packageName) || 0) + 1)
      }
      const packages: PackageInfo[] = Array.from(packageMap.entries())
        .map(([name, classCount]) => ({ name, classCount }))
        .sort((a, b) => a.name.localeCompare(b.name))

      return {
        jarPath,
        jarName,
        metadata,
        classNames,
        packageNames,
        packages,
        classes,
        configFiles,
        dependencies,
        detectedTechs,
        entryCount: classes.length + configFiles.length,
        sizeBytes: stat,
      }
    } finally {
      // Cleanup temp directory
      await rm(tempDir, { recursive: true, force: true }).catch(() => {})
    }
  }

  /**
   * Extract JAR file to directory
   */
  async function extractJar(jarPath: string, destDir: string): Promise<void> {
    // Use system commands to extract JAR file
    // JAR files are ZIP archives, so we can use unzip or jar commands
    const { execSync } = require("node:child_process")

    try {
      // Try using unzip command first (most common)
      execSync(`unzip -q "${jarPath}" -d "${destDir}"`, { stdio: "ignore" })
    } catch {
      // Fallback: use jar command (part of JDK)
      try {
        execSync(`cd "${destDir}" && jar xf "${jarPath}"`, { stdio: "ignore" })
      } catch {
        throw new Error("Failed to extract JAR file. Please ensure 'unzip' or 'jar' command is available.")
      }
    }
  }

  /**
   * Extract metadata from META-INF/MANIFEST.MF
   */
  async function extractMetadata(extractDir: string): Promise<JarMetadata> {
    const manifestPath = path.join(extractDir, MANIFEST_PATH)

    if (!(await exists(manifestPath))) {
      return {}
    }

    const manifestContent = await Bun.file(manifestPath).text()
    const metadata: JarMetadata = {}

    const manifestEntries = manifestContent.split(/\r?\n/)

    let currentKey = ""
    for (const line of manifestEntries) {
      // Continuation lines start with space
      if (line.startsWith(" ") && currentKey) {
        continue
      }

      const colonIndex = line.indexOf(":")
      if (colonIndex > 0) {
        currentKey = line.substring(0, colonIndex).trim()
        const value = line.substring(colonIndex + 1).trim()

        switch (currentKey) {
          case "Main-Class":
            metadata.mainClass = value
            break
          case "Implementation-Title":
            metadata.implementationTitle = value
            break
          case "Implementation-Version":
            metadata.implementationVersion = value
            break
          case "Implementation-Vendor":
            metadata.implementationVendor = value
            break
          case "Specification-Title":
            metadata.specificationTitle = value
            break
          case "Specification-Version":
            metadata.specificationVersion = value
            break
          case "Created-By":
            metadata.createdBy = value
            break
          case "Build-Jdk":
          case "Build-Jdk-Spec":
            metadata.jdkVersion = value
            break
          case "Bundle-Name":
            metadata.bundleName = value
            break
          case "Bundle-Version":
            metadata.bundleVersion = value
            break
          case "Bundle-SymbolicName":
            metadata.bundleSymbolicName = value
            break
          case "Bundle-ManifestVersion":
            metadata.buildTool = "OSGi"
            break
          case "Archiver-Version":
            if (value.includes("Maven")) metadata.buildTool = "Maven"
            break
          case "Gradle-Version":
            metadata.buildTool = "Gradle"
            break
        }
      }
    }

    // Detect build tool from structure
    if (!metadata.buildTool) {
      const pomPath = path.join(extractDir, "META-INF", "maven")
      if (await exists(pomPath)) {
        metadata.buildTool = "Maven"
      }
    }

    return metadata
  }

  /**
   * Extract class information from class files
   */
  async function extractClasses(extractDir: string, maxClasses: number): Promise<ClassInfo[]> {
    const classes: ClassInfo[] = []

    // Recursively find all .class files
    async function findClassFiles(dir: string, baseDir: string = dir) {
      // Use glob to find class files
      const glob = new Bun.Glob("**/*.class")
      const classFiles: string[] = []

      for await (const file of glob.scan({ cwd: extractDir, absolute: true })) {
        classFiles.push(file)
        if (classFiles.length >= maxClasses) break
      }

      return classFiles
    }

    const classFiles = await findClassFiles(extractDir)

    for (const classFilePath of classFiles.slice(0, maxClasses)) {
      const classInfo = await parseClassFile(classFilePath, extractDir)
      if (classInfo) {
        classes.push(classInfo)
      }
    }

    return classes.sort((a, b) => a.name.localeCompare(b.name))
  }

  /**
   * Parse a single class file
   */
  async function parseClassFile(classFilePath: string, baseDir: string): Promise<ClassInfo | null> {
    try {
      const buffer = await Bun.file(classFilePath).arrayBuffer()
      const view = new DataView(buffer)

      // Verify magic number (CAFEBABE)
      if (view.getUint32(0, false) !== 0xCAFEBABE) {
        return null
      }

      // Get minor and major version
      const minorVersion = view.getUint16(4, false)
      const majorVersion = view.getUint16(6, false)

      const javaVersion = BYTECODE_VERSION_MAP[majorVersion] || `${majorVersion}.0`

      // Get access flags (modifiers)
      const accessFlags = view.getUint16(8, false)

      // Get this_class index (points to constant pool)
      const thisClass = view.getUint16(10, false)

      // Read constant pool to get class name
      const constantPoolCount = view.getUint16(12, false)
      let offset = 14

      // Parse constant pool
      const constantPool: (string | null)[] = [null] // Index 0 is null

      for (let i = 1; i < constantPoolCount; i++) {
        const tag = view.getUint8(offset)
        offset++

        switch (tag) {
          case 1: // CONSTANT_Utf8
            const length = view.getUint16(offset, false)
            offset += 2
            const bytes = new Uint8Array(buffer, offset, length)
            const str = new TextDecoder("latin1").decode(bytes)
            constantPool[i] = str
            offset += length
            break
          case 7: // CONSTANT_Class
          case 8: // CONSTANT_String
            offset += 2
            break
          case 3: // CONSTANT_Integer
          case 4: // CONSTANT_Float
          case 9: // CONSTANT_Fieldref
          case 10: // CONSTANT_Methodref
          case 11: // CONSTANT_InterfaceMethodref
          case 12: // CONSTANT_NameAndType
            offset += 4
            break
          case 5: // CONSTANT_Long
          case 6: // CONSTANT_Double
            offset += 8
            i++ // These take two slots
            break
          case 15: // CONSTANT_MethodHandle
            offset += 3
            break
          case 16: // CONSTANT_MethodType
            offset += 2
            break
          case 18: // CONSTANT_InvokeDynamic
            offset += 4
            break
          default:
            break
        }
      }

      // Get class name from file path
      // Note: Full constant pool parsing would be needed for accurate class name
      const classPath = classFilePath.substring(baseDir.length + 1)
      const className = classPath.replace(/\//g, ".").replace(/\.class$/, "")

      // Determine class type and modifiers from access flags
      const isAnnotation = (accessFlags & 0x2000) !== 0
      const isEnum = (accessFlags & 0x4000) !== 0
      const isInterface = (accessFlags & 0x0200) !== 0
      const isPublic = (accessFlags & 0x0001) !== 0
      const isFinal = (accessFlags & 0x0010) !== 0
      const isAbstract = (accessFlags & 0x0400) !== 0

      const type: ClassInfo["type"] = isAnnotation ? "annotation" : isEnum ? "enum" : isInterface ? "interface" : "class"

      const modifiers: string[] = []
      if (isPublic) modifiers.push("public")
      if (isFinal) modifiers.push("final")
      if (isAbstract && !isInterface) modifiers.push("abstract")

      // Convert class name to dot notation
      const dotName = className.replace(/\//g, ".")
      const lastDotIndex = dotName.lastIndexOf(".")

      return {
        name: dotName,
        packageName: lastDotIndex >= 0 ? dotName.substring(0, lastDotIndex) : "",
        simpleName: lastDotIndex >= 0 ? dotName.substring(lastDotIndex + 1) : dotName,
        type,
        modifiers,
        bytecodeVersion: majorVersion,
        javaVersion,
      }
    } catch {
      return null
    }
  }

  /**
   * Extract configuration files
   */
  async function extractConfigFiles(extractDir: string): Promise<ConfigFile[]> {
    const configFiles: ConfigFile[] = []

    const glob = new Bun.Glob("**/*.{properties,yml,yaml,xml}")
    for await (const filePath of glob.scan({ cwd: extractDir, absolute: true })) {
      // Skip files in META-INF/maven (they're dependency info)
      if (filePath.includes("/META-INF/maven/")) continue

      // Skip class-related files in META-INF
      if (filePath.includes("/META-INF/")) {
        const fileName = path.basename(filePath)
        if (fileName.endsWith(".SF") || fileName.endsWith(".DSA") || fileName.endsWith(".RSA")) {
          continue // Skip signature files
        }
      }

      const ext = path.extname(filePath)
      const fileName = path.basename(filePath)
      const relativePath = filePath.substring(extractDir.length + 1)

      let type = "unknown"
      if (ext === ".properties") type = "properties"
      else if (ext === ".yml" || ext === ".yaml") type = "yaml"
      else if (ext === ".xml") {
        if (fileName.startsWith("spring") || fileName === "applicationContext.xml") type = "spring-config"
        else if (fileName === "web.xml") type = "web-config"
        else if (fileName === "persistence.xml") type = "jpa-config"
        else if (fileName.startsWith("hibernate")) type = "hibernate-config"
        else type = "xml"
      }

      // Read small files
      let content: string | undefined
      try {
        const file = Bun.file(filePath)
        const fileSize = file.size
        if (fileSize < 10000) {
          content = await file.text()
        }
      } catch {
        // File read failed, skip content
      }

      configFiles.push({
        path: relativePath,
        type,
        content,
      })
    }

    return configFiles.sort((a, b) => a.path.localeCompare(b.path))
  }

  /**
   * Extract dependencies from Maven metadata
   */
  async function extractDependencies(extractDir: string): Promise<Dependency[]> {
    const dependencies: Dependency[] = []
    const mavenMetaPath = path.join(extractDir, "META-INF", "maven")

    if (!(await exists(mavenMetaPath))) {
      return dependencies
    }

    // Find all pom.properties files
    const glob = new Bun.Glob("**/pom.properties")
    for await (const filePath of glob.scan({ cwd: mavenMetaPath, absolute: true })) {
      try {
        const content = await Bun.file(filePath).text()
        const props = parseProperties(content)

        if (props.groupId && props.artifactId) {
          dependencies.push({
            groupId: props.groupId as string,
            artifactId: props.artifactId as string,
            version: props.version as string,
          })
        }
      } catch {
        // Skip invalid files
      }
    }

    return dependencies
  }

  /**
   * Parse Java .properties file content
   */
  function parseProperties(content: string): Record<string, string> {
    const props: Record<string, string> = {}

    for (const line of content.split(/\r?\n/)) {
      const trimmed = line.trim()
      if (!trimmed || trimmed.startsWith("#") || trimmed.startsWith("!")) continue

      const eqIndex = trimmed.indexOf("=")
      const colonIndex = trimmed.indexOf(":")

      const sepIndex = eqIndex >= 0 && (colonIndex < 0 || eqIndex < colonIndex) ? eqIndex : colonIndex

      if (sepIndex > 0) {
        const key = trimmed.substring(0, sepIndex).trim()
        const value = trimmed.substring(sepIndex + 1).trim()
        props[key] = value
      }
    }

    return props
  }

  /**
   * Get a summary of the JAR analysis
   */
  export function getSummary(result: JarAnalysisResult): string {
    const lines: string[] = []

    lines.push(`JAR: ${result.jarName}`)
    lines.push(`Size: ${formatBytes(result.sizeBytes)}`)
    lines.push(`Entries: ${result.entryCount}`)
    lines.push(`Classes: ${result.classes.length}`)
    lines.push(`Packages: ${result.packages.length}`)

    if (result.metadata.mainClass) {
      lines.push(`Main Class: ${result.metadata.mainClass}`)
    }

    if (result.metadata.buildTool) {
      lines.push(`Build Tool: ${result.metadata.buildTool}`)
    }

    if (result.metadata.implementationVersion) {
      lines.push(`Version: ${result.metadata.implementationVersion}`)
    }

    const techCount = result.detectedTechs.size
    if (techCount > 0) {
      lines.push(`Detected Technologies: ${techCount}`)
    }

    return lines.join("\n")
  }
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B"
  const k = 1024
  const sizes = ["B", "KB", "MB", "GB"]
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`
}
