/**
 * Java Technology Fingerprint Database
 *
 * Comprehensive fingerprint patterns for detecting Java frameworks and libraries
 * from JAR files, class names, and package structures.
 */

export interface JavaFingerprintPattern {
  pattern: string | RegExp | string[]
  type: "class" | "package" | "config" | "annotation" | "manifest" | "dependency"
  confidence: "high" | "medium" | "low"
  notes?: string
}

export interface JavaFingerprint {
  name: string
  patterns: JavaFingerprintPattern[]
  category: string
  website?: string
}

export const JAVA_FINGERPRINTS: Record<string, JavaFingerprint[]> = {
  // ============================================
  // FRAMEWORKS
  // ============================================
  framework: [
    {
      name: "Spring Boot",
      category: "framework",
      website: "https://spring.io/projects/spring-boot",
      patterns: [
        { pattern: "org.springframework.boot", type: "package", confidence: "high" },
        { pattern: "spring-boot", type: "manifest", confidence: "high" },
        { pattern: "SpringApplication", type: "class", confidence: "high" },
        { pattern: "@SpringBootApplication", type: "annotation", confidence: "high" },
        { pattern: "application.properties", type: "config", confidence: "medium" },
        { pattern: "application.yml", type: "config", confidence: "medium" },
        { pattern: "META-INF/spring.factories", type: "config", confidence: "high" },
      ],
    },
    {
      name: "Spring Framework",
      category: "framework",
      website: "https://spring.io/projects/spring-framework",
      patterns: [
        { pattern: "org.springframework", type: "package", confidence: "high" },
        { pattern: "@Controller", type: "annotation", confidence: "high" },
        { pattern: "@RestController", type: "annotation", confidence: "high" },
        { pattern: "@Service", type: "annotation", confidence: "high" },
        { pattern: "@Repository", type: "annotation", confidence: "high" },
        { pattern: "@Component", type: "annotation", confidence: "high" },
        { pattern: "ApplicationContext", type: "class", confidence: "high" },
        { pattern: "spring-", type: "manifest", confidence: "medium" },
      ],
    },
    {
      name: "Jakarta EE",
      category: "framework",
      website: "https://jakarta.ee",
      patterns: [
        { pattern: "jakarta.", type: "package", confidence: "high" },
        { pattern: "javax.", type: "package", confidence: "medium", notes: "Legacy Java EE" },
        { pattern: "@WebServlet", type: "annotation", confidence: "high" },
        { pattern: "@EJB", type: "annotation", confidence: "high" },
        { pattern: "@Stateless", type: "annotation", confidence: "medium" },
        { pattern: "@Stateful", type: "annotation", confidence: "medium" },
        { pattern: "web.xml", type: "config", confidence: "medium" },
      ],
    },
    {
      name: "Micronaut",
      category: "framework",
      website: "https://micronaut.io",
      patterns: [
        { pattern: "io.micronaut", type: "package", confidence: "high" },
        { pattern: "@Controller", type: "annotation", confidence: "medium" },
        { pattern: "@Client", type: "annotation", confidence: "medium" },
        { pattern: "micronaut-", type: "manifest", confidence: "high" },
      ],
    },
    {
      name: "Quarkus",
      category: "framework",
      website: "https://quarkus.io",
      patterns: [
        { pattern: "io.quarkus", type: "package", confidence: "high" },
        { pattern: "quarkus-", type: "manifest", confidence: "high" },
        { pattern: "@Path", type: "annotation", confidence: "medium" },
      ],
    },
    {
      name: "Google Guice",
      category: "framework",
      website: "https://github.com/google/guice",
      patterns: [
        { pattern: "com.google.inject", type: "package", confidence: "high" },
        { pattern: "@Inject", type: "annotation", confidence: "medium" },
        { pattern: "Injector", type: "class", confidence: "high" },
        { pattern: "Module", type: "class", confidence: "medium" },
      ],
    },
    {
      name: "JavaServer Faces (JSF)",
      category: "framework",
      website: "https://jakarta.ee/specifications/faces/",
      patterns: [
        { pattern: "jakarta.faces", type: "package", confidence: "high" },
        { pattern: "javax.faces", type: "package", confidence: "medium" },
        { pattern: "@ManagedBean", type: "annotation", confidence: "high" },
        { pattern: "faces-config.xml", type: "config", confidence: "high" },
      ],
    },
    {
      name: "Vaadin",
      category: "framework",
      website: "https://vaadin.com",
      patterns: [
        { pattern: "com.vaadin", type: "package", confidence: "high" },
        { pattern: "vaadin-", type: "manifest", confidence: "high" },
      ],
    },
    {
      name: "Apache Wicket",
      category: "framework",
      website: "https://wicket.apache.org",
      patterns: [
        { pattern: "org.apache.wicket", type: "package", confidence: "high" },
        { pattern: "wicket-", type: "manifest", confidence: "high" },
      ],
    },
    {
      name: "Struts",
      category: "framework",
      website: "https://struts.apache.org",
      patterns: [
        { pattern: "org.apache.struts", type: "package", confidence: "high" },
        { pattern: "struts-", type: "manifest", confidence: "high" },
        { pattern: "struts.xml", type: "config", confidence: "high" },
      ],
    },
  ],

  // ============================================
  // ORM / DATABASE
  // ============================================
  orm: [
    {
      name: "Hibernate",
      category: "orm",
      website: "https://hibernate.org",
      patterns: [
        { pattern: "org.hibernate", type: "package", confidence: "high" },
        { pattern: "@Entity", type: "annotation", confidence: "medium" },
        { pattern: "@Table", type: "annotation", confidence: "medium" },
        { pattern: "Session", type: "class", confidence: "medium" },
        { pattern: "hibernate.cfg.xml", type: "config", confidence: "high" },
        { pattern: "hibernate-", type: "manifest", confidence: "high" },
      ],
    },
    {
      name: "EclipseLink",
      category: "orm",
      website: "https://www.eclipse.org/eclipselink",
      patterns: [
        { pattern: "org.eclipse.persistence", type: "package", confidence: "high" },
        { pattern: "eclipselink-", type: "manifest", confidence: "high" },
        { pattern: "persistence.xml", type: "config", confidence: "medium" },
      ],
    },
    {
      name: "MyBatis",
      category: "orm",
      website: "https://mybatis.org",
      patterns: [
        { pattern: "org.apache.ibatis", type: "package", confidence: "high" },
        { pattern: "mybatis-", type: "manifest", confidence: "high" },
        { pattern: "mybatis-config.xml", type: "config", confidence: "high" },
        { pattern: "*Mapper.xml", type: "config", confidence: "high" },
        { pattern: "@Mapper", type: "annotation", confidence: "medium" },
      ],
    },
    {
      name: "JOOQ",
      category: "orm",
      website: "https://www.jooq.org",
      patterns: [
        { pattern: "org.jooq", type: "package", confidence: "high" },
        { pattern: "jooq-", type: "manifest", confidence: "high" },
      ],
    },
    {
      name: "Spring Data",
      category: "orm",
      website: "https://spring.io/projects/spring-data",
      patterns: [
        { pattern: "org.springframework.data", type: "package", confidence: "high" },
        { pattern: "CrudRepository", type: "class", confidence: "high" },
        { pattern: "JpaRepository", type: "class", confidence: "high" },
        { pattern: "@Repository", type: "annotation", confidence: "medium" },
      ],
    },
    {
      name: "Apache Cayenne",
      category: "orm",
      website: "https://cayenne.apache.org",
      patterns: [
        { pattern: "org.apache.cayenne", type: "package", confidence: "high" },
        { pattern: "cayenne-", type: "manifest", confidence: "high" },
        { pattern: "cayenne.xml", type: "config", confidence: "high" },
      ],
    },
    {
      name: "ObjectDB",
      category: "orm",
      website: "https://www.objectdb.com",
      patterns: [
        { pattern: "com.objectdb", type: "package", confidence: "high" },
        { pattern: "objectdb", type: "manifest", confidence: "high" },
      ],
    },
  ],

  // ============================================
  // WEB SERVERS
  // ============================================
  web: [
    {
      name: "Apache Tomcat",
      category: "web",
      website: "https://tomcat.apache.org",
      patterns: [
        { pattern: "org.apache.catalina", type: "package", confidence: "high" },
        { pattern: "org.apache.tomcat", type: "package", confidence: "high" },
        { pattern: "tomcat-", type: "manifest", confidence: "high" },
        { pattern: "catalina.", type: "config", confidence: "medium" },
      ],
    },
    {
      name: "Jetty",
      category: "web",
      website: "https://www.eclipse.org/jetty",
      patterns: [
        { pattern: "org.eclipse.jetty", type: "package", confidence: "high" },
        { pattern: "jetty-", type: "manifest", confidence: "high" },
      ],
    },
    {
      name: "Netty",
      category: "web",
      website: "https://netty.io",
      patterns: [
        { pattern: "io.netty", type: "package", confidence: "high" },
        { pattern: "netty-", type: "manifest", confidence: "high" },
        { pattern: "ByteBuf", type: "class", confidence: "medium" },
        { pattern: "Channel", type: "class", confidence: "medium" },
      ],
    },
    {
      name: "Undertow",
      category: "web",
      website: "https://undertow.io",
      patterns: [
        { pattern: "io.undertow", type: "package", confidence: "high" },
        { pattern: "undertow-", type: "manifest", confidence: "high" },
      ],
    },
  ],

  // ============================================
  // SERIALIZATION / JSON
  // ============================================
  serialization: [
    {
      name: "Jackson",
      category: "serialization",
      website: "https://github.com/FasterXML/jackson",
      patterns: [
        { pattern: "com.fasterxml.jackson", type: "package", confidence: "high" },
        { pattern: "jackson-", type: "manifest", confidence: "high" },
        { pattern: "ObjectMapper", type: "class", confidence: "high" },
        { pattern: "@JsonProperty", type: "annotation", confidence: "medium" },
        { pattern: "@JsonFormat", type: "annotation", confidence: "medium" },
      ],
    },
    {
      name: "Gson",
      category: "serialization",
      website: "https://github.com/google/gson",
      patterns: [
        { pattern: "com.google.gson", type: "package", confidence: "high" },
        { pattern: "gson-", type: "manifest", confidence: "high" },
        { pattern: "Gson", type: "class", confidence: "high" },
        { pattern: "JsonElement", type: "class", confidence: "medium" },
        { pattern: "@SerializedName", type: "annotation", confidence: "high" },
      ],
    },
    {
      name: "JSON-B",
      category: "serialization",
      website: "https://eclipse-ee4j.github.io/jsonb-api",
      patterns: [
        { pattern: "jakarta.json.bind", type: "package", confidence: "high" },
        { pattern: "javax.json.bind", type: "package", confidence: "medium" },
        { pattern: "Jsonb", type: "class", confidence: "high" },
        { pattern: "@JsonbProperty", type: "annotation", confidence: "medium" },
      ],
    },
    {
      name: "FastJSON",
      category: "serialization",
      website: "https://github.com/alibaba/fastjson",
      patterns: [
        { pattern: "com.alibaba.fastjson", type: "package", confidence: "high" },
        { pattern: "fastjson-", type: "manifest", confidence: "high" },
        { pattern: "JSON", type: "class", confidence: "medium" },
        { pattern: "@JSONField", type: "annotation", confidence: "high" },
      ],
    },
  ],

  // ============================================
  // UTILITIES
  // ============================================
  utility: [
    {
      name: "Apache Commons",
      category: "utility",
      website: "https://commons.apache.org",
      patterns: [
        { pattern: "org.apache.commons", type: "package", confidence: "high" },
        { pattern: "commons-", type: "manifest", confidence: "high" },
      ],
    },
    {
      name: "Google Guava",
      category: "utility",
      website: "https://github.com/google/guava",
      patterns: [
        { pattern: "com.google.common", type: "package", confidence: "high" },
        { pattern: "guava-", type: "manifest", confidence: "high" },
        { pattern: "ImmutableList", type: "class", confidence: "medium" },
        { pattern: "Optional", type: "class", confidence: "low" },
      ],
    },
    {
      name: "Lombok",
      category: "utility",
      website: "https://projectlombok.org",
      patterns: [
        { pattern: "lombok", type: "manifest", confidence: "high" },
        { pattern: "@Data", type: "annotation", confidence: "high" },
        { pattern: "@Getter", type: "annotation", confidence: "high" },
        { pattern: "@Setter", type: "annotation", confidence: "high" },
        { pattern: "@Builder", type: "annotation", confidence: "high" },
        { pattern: "@Slf4j", type: "annotation", confidence: "high" },
        { pattern: "@AllArgsConstructor", type: "annotation", confidence: "medium" },
        { pattern: "@NoArgsConstructor", type: "annotation", confidence: "medium" },
      ],
    },
    {
      name: "Eclipse Collections",
      category: "utility",
      website: "https://www.eclipse.org/collections",
      patterns: [
        { pattern: "org.eclipse.collections", type: "package", confidence: "high" },
        { pattern: "eclipse-collections", type: "manifest", confidence: "high" },
      ],
    },
    {
      name: "Vavr",
      category: "utility",
      website: "https://www.vavr.io",
      patterns: [
        { pattern: "io.vavr", type: "package", confidence: "high" },
        { pattern: "vavr-", type: "manifest", confidence: "high" },
      ],
    },
    {
      name: "Apache POI",
      category: "utility",
      website: "https://poi.apache.org",
      patterns: [
        { pattern: "org.apache.poi", type: "package", confidence: "high" },
        { pattern: "poi-", type: "manifest", confidence: "high" },
      ],
    },
    {
      name: "MapDB",
      category: "utility",
      website: "https://mapdb.org",
      patterns: [
        { pattern: "org.mapdb", type: "package", confidence: "high" },
        { pattern: "mapdb-", type: "manifest", confidence: "high" },
      ],
    },
  ],

  // ============================================
  // LOGGING
  // ============================================
  logging: [
    {
      name: "SLF4J",
      category: "logging",
      website: "http://www.slf4j.org",
      patterns: [
        { pattern: "org.slf4j", type: "package", confidence: "high" },
        { pattern: "slf4j-", type: "manifest", confidence: "high" },
        { pattern: "Logger", type: "class", confidence: "low" },
        { pattern: "LoggerFactory", type: "class", confidence: "high" },
      ],
    },
    {
      name: "Logback",
      category: "logging",
      website: "https://logback.qos.ch",
      patterns: [
        { pattern: "ch.qos.logback", type: "package", confidence: "high" },
        { pattern: "logback-", type: "manifest", confidence: "high" },
        { pattern: "logback.xml", type: "config", confidence: "high" },
        { pattern: "logback-.xml", type: "config", confidence: "medium" },
      ],
    },
    {
      name: "Log4j",
      category: "logging",
      website: "https://logging.apache.org/log4j",
      patterns: [
        { pattern: "org.apache.logging.log4j", type: "package", confidence: "high" },
        { pattern: "log4j-", type: "manifest", confidence: "high" },
        { pattern: "log4j2.xml", type: "config", confidence: "high" },
        { pattern: "log4j.properties", type: "config", confidence: "high" },
        { pattern: "log4j.xml", type: "config", confidence: "medium" },
      ],
    },
    {
      name: "Apache Commons Logging",
      category: "logging",
      website: "https://commons.apache.org/proper/commons-logging",
      patterns: [
        { pattern: "org.apache.commons.logging", type: "package", confidence: "high" },
        { pattern: "commons-logging", type: "manifest", confidence: "high" },
      ],
    },
    {
      name: "Java Util Logging (JUL)",
      category: "logging",
      website: "https://docs.oracle.com/javase/8/docs/technotes/guides/logging",
      patterns: [
        { pattern: "java.util.logging", type: "package", confidence: "high" },
        { pattern: "logging.properties", type: "config", confidence: "medium" },
      ],
    },
  ],

  // ============================================
  // TESTING
  // ============================================
  testing: [
    {
      name: "JUnit",
      category: "testing",
      website: "https://junit.org",
      patterns: [
        { pattern: "org.junit", type: "package", confidence: "high" },
        { pattern: "junit-", type: "manifest", confidence: "high" },
        { pattern: "@Test", type: "annotation", confidence: "medium" },
        { pattern: "@BeforeEach", type: "annotation", confidence: "medium" },
        { pattern: "@BeforeAll", type: "annotation", confidence: "medium" },
        { pattern: "@DisplayName", type: "annotation", confidence: "high" },
      ],
    },
    {
      name: "TestNG",
      category: "testing",
      website: "https://testng.org",
      patterns: [
        { pattern: "org.testng", type: "package", confidence: "high" },
        { pattern: "testng-", type: "manifest", confidence: "high" },
        { pattern: "@Test", type: "annotation", confidence: "low" },
        { pattern: "@BeforeMethod", type: "annotation", confidence: "high" },
        { pattern: "@DataProvider", type: "annotation", confidence: "high" },
        { pattern: "testng.xml", type: "config", confidence: "high" },
      ],
    },
    {
      name: "Mockito",
      category: "testing",
      website: "https://site.mockito.org",
      patterns: [
        { pattern: "org.mockito", type: "package", confidence: "high" },
        { pattern: "mockito-", type: "manifest", confidence: "high" },
        { pattern: "@Mock", type: "annotation", confidence: "medium" },
        { pattern: "@Spy", type: "annotation", confidence: "medium" },
        { pattern: "@InjectMocks", type: "annotation", confidence: "high" },
        { pattern: "Mockito.", type: "class", confidence: "high" },
      ],
    },
    {
      name: "AssertJ",
      category: "testing",
      website: "https://assertj.github.io/doc",
      patterns: [
        { pattern: "org.assertj", type: "package", confidence: "high" },
        { pattern: "assertj-", type: "manifest", confidence: "high" },
        { pattern: "Assertions", type: "class", confidence: "medium" },
      ],
    },
    {
      name: "Hamcrest",
      category: "testing",
      website: "https://hamcrest.org",
      patterns: [
        { pattern: "org.hamcrest", type: "package", confidence: "high" },
        { pattern: "hamcrest-", type: "manifest", confidence: "high" },
      ],
    },
    {
      name: "Selenium",
      category: "testing",
      website: "https://www.selenium.dev",
      patterns: [
        { pattern: "org.openqa.selenium", type: "package", confidence: "high" },
        { pattern: "selenium-", type: "manifest", confidence: "high" },
        { pattern: "WebDriver", type: "class", confidence: "high" },
      ],
    },
    {
      name: "Cucumber",
      category: "testing",
      website: "https://cucumber.io",
      patterns: [
        { pattern: "io.cucumber", type: "package", confidence: "high" },
        { pattern: "cucumber-", type: "manifest", confidence: "high" },
        { pattern: "@Given", type: "annotation", confidence: "high" },
        { pattern: "@When", type: "annotation", confidence: "high" },
        { pattern: "@Then", type: "annotation", confidence: "high" },
        { pattern: ".feature", type: "config", confidence: "medium" },
      ],
    },
  ],

  // ============================================
  // MESSAGING
  // ============================================
  messaging: [
    {
      name: "Apache Kafka",
      category: "messaging",
      website: "https://kafka.apache.org",
      patterns: [
        { pattern: "org.apache.kafka", type: "package", confidence: "high" },
        { pattern: "kafka_", type: "manifest", confidence: "high" },
        { pattern: "kafka-", type: "manifest", confidence: "high" },
        { pattern: "@KafkaListener", type: "annotation", confidence: "high" },
      ],
    },
    {
      name: "RabbitMQ",
      category: "messaging",
      website: "https://www.rabbitmq.com",
      patterns: [
        { pattern: "com.rabbitmq", type: "package", confidence: "high" },
        { pattern: "amqp-", type: "manifest", confidence: "high" },
        { pattern: "@RabbitListener", type: "annotation", confidence: "high" },
        { pattern: "@RabbitHandler", type: "annotation", confidence: "high" },
      ],
    },
    {
      name: "ActiveMQ",
      category: "messaging",
      website: "https://activemq.apache.org",
      patterns: [
        { pattern: "org.apache.activemq", type: "package", confidence: "high" },
        { pattern: "activemq-", type: "manifest", confidence: "high" },
        { pattern: "@JmsListener", type: "annotation", confidence: "medium" },
      ],
    },
    {
      name: "Apache Pulsar",
      category: "messaging",
      website: "https://pulsar.apache.org",
      patterns: [
        { pattern: "org.apache.pulsar", type: "package", confidence: "high" },
        { pattern: "pulsar-", type: "manifest", confidence: "high" },
      ],
    },
  ],

  // ============================================
  // CACHING
  // ============================================
  caching: [
    {
      name: "Caffeine",
      category: "caching",
      website: "https://github.com/ben-manes/caffeine",
      patterns: [
        { pattern: "com.github.benmanes.caffeine", type: "package", confidence: "high" },
        { pattern: "caffeine-", type: "manifest", confidence: "high" },
        { pattern: "Cache", type: "class", confidence: "low" },
      ],
    },
    {
      name: "Ehcache",
      category: "caching",
      website: "https://www.ehcache.org",
      patterns: [
        { pattern: "org.ehcache", type: "package", confidence: "high" },
        { pattern: "net.sf.ehcache", type: "package", confidence: "medium" },
        { pattern: "ehcache-", type: "manifest", confidence: "high" },
        { pattern: "ehcache.xml", type: "config", confidence: "high" },
      ],
    },
    {
      name: "Redis (Lettuce/Jedis)",
      category: "caching",
      website: "https://redis.io",
      patterns: [
        { pattern: "io.lettuce", type: "package", confidence: "high" },
        { pattern: "redis.clients.jedis", type: "package", confidence: "high" },
        { pattern: "lettuce-", type: "manifest", confidence: "high" },
        { pattern: "jedis-", type: "manifest", confidence: "high" },
      ],
    },
    {
      name: "Hazelcast",
      category: "caching",
      website: "https://hazelcast.com",
      patterns: [
        { pattern: "com.hazelcast", type: "package", confidence: "high" },
        { pattern: "hazelcast-", type: "manifest", confidence: "high" },
        { pattern: "hazelcast.xml", type: "config", confidence: "high" },
      ],
    },
    {
      name: "Infinispan",
      category: "caching",
      website: "https://infinispan.org",
      patterns: [
        { pattern: "org.infinispan", type: "package", confidence: "high" },
        { pattern: "infinispan-", type: "manifest", confidence: "high" },
      ],
    },
  ],

  // ============================================
  // VALIDATION
  // ============================================
  validation: [
    {
      name: "Hibernate Validator",
      category: "validation",
      website: "https://hibernate.org/validator",
      patterns: [
        { pattern: "org.hibernate.validator", type: "package", confidence: "high" },
        { pattern: "@NotNull", type: "annotation", confidence: "medium" },
        { pattern: "@NotEmpty", type: "annotation", confidence: "medium" },
        { pattern: "@Size", type: "annotation", confidence: "medium" },
        { pattern: "@Min", type: "annotation", confidence: "medium" },
        { pattern: "@Max", type: "annotation", confidence: "medium" },
        { pattern: "@Pattern", type: "annotation", confidence: "medium" },
        { pattern: "@Email", type: "annotation", confidence: "medium" },
        { pattern: "hibernate-validator-", type: "manifest", confidence: "high" },
      ],
    },
  ],

  // ============================================
  // SECURITY
  // ============================================
  security: [
    {
      name: "Spring Security",
      category: "security",
      website: "https://spring.io/projects/spring-security",
      patterns: [
        { pattern: "org.springframework.security", type: "package", confidence: "high" },
        { pattern: "spring-security-", type: "manifest", confidence: "high" },
        { pattern: "@PreAuthorize", type: "annotation", confidence: "high" },
        { pattern: "@Secured", type: "annotation", confidence: "high" },
        { pattern: "@EnableWebSecurity", type: "annotation", confidence: "high" },
      ],
    },
    {
      name: "Apache Shiro",
      category: "security",
      website: "https://shiro.apache.org",
      patterns: [
        { pattern: "org.apache.shiro", type: "package", confidence: "high" },
        { pattern: "shiro-", type: "manifest", confidence: "high" },
        { pattern: "@RequiresAuthentication", type: "annotation", confidence: "high" },
      ],
    },
    {
      name: "JWT (java-jwt/jjwt)",
      category: "security",
      website: "https://github.com/auth0/java-jwt",
      patterns: [
        { pattern: "com.auth0.jwt", type: "package", confidence: "high" },
        { pattern: "io.jsonwebtoken", type: "package", confidence: "high" },
        { pattern: "jwt-", type: "manifest", confidence: "high" },
      ],
    },
  ],

  // ============================================
  // SCHEDULING
  // ============================================
  scheduling: [
    {
      name: "Quartz",
      category: "scheduling",
      website: "http://www.quartz-scheduler.org",
      patterns: [
        { pattern: "org.quartz", type: "package", confidence: "high" },
        { pattern: "quartz-", type: "manifest", confidence: "high" },
        { pattern: "Job", type: "class", confidence: "medium" },
        { pattern: "@DisallowConcurrentExecution", type: "annotation", confidence: "medium" },
      ],
    },
  ],

  // ============================================
  // HTTP CLIENTS
  // ============================================
  http: [
    {
      name: "OkHttp",
      category: "http",
      website: "https://square.github.io/okhttp",
      patterns: [
        { pattern: "okhttp3", type: "package", confidence: "high" },
        { pattern: "okhttp-", type: "manifest", confidence: "high" },
        { pattern: "OkHttpClient", type: "class", confidence: "high" },
      ],
    },
    {
      name: "Apache HttpClient",
      category: "http",
      website: "https://hc.apache.org/httpcomponents-client",
      patterns: [
        { pattern: "org.apache.http", type: "package", confidence: "high" },
        { pattern: "httpclient-", type: "manifest", confidence: "high" },
        { pattern: "HttpClient", type: "class", confidence: "low" },
      ],
    },
    {
      name: "Retrofit",
      category: "http",
      website: "https://square.github.io/retrofit",
      patterns: [
        { pattern: "retrofit2", type: "package", confidence: "high" },
        { pattern: "retrofit-", type: "manifest", confidence: "high" },
        { pattern: "@GET", type: "annotation", confidence: "medium" },
        { pattern: "@POST", type: "annotation", confidence: "medium" },
      ],
    },
    {
      name: "Feign",
      category: "http",
      website: "https://github.com/OpenFeign/feign",
      patterns: [
        { pattern: "feign", type: "package", confidence: "high" },
        { pattern: "@FeignClient", type: "annotation", confidence: "high" },
      ],
    },
  ],
}

/**
 * Find Java technologies matching given content
 */
export function findJavaFingerprints(content: {
  classNames?: string[]
  packageNames?: string[]
  configFiles?: string[]
  manifest?: Record<string, string>
  annotations?: string[]
}): Map<string, { tech: JavaFingerprint; matches: string[] }> {
  const results = new Map<string, { tech: JavaFingerprint; matches: string[] }>()

  const allStrings = [
    ...(content.classNames || []),
    ...(content.packageNames || []),
    ...(content.configFiles || []),
    ...(content.annotations || []),
    ...(content.manifest ? Object.values(content.manifest) : []),
  ].join("\n").toLowerCase()

  for (const [category, techs] of Object.entries(JAVA_FINGERPRINTS)) {
    for (const tech of techs) {
      const matches: string[] = []

      for (const pattern of tech.patterns) {
        const patterns = Array.isArray(pattern.pattern) ? pattern.pattern : [pattern.pattern]
        const targetContent =
          pattern.type === "manifest"
            ? allStrings
            : pattern.type === "package"
              ? (content.packageNames || []).join("\n")
              : pattern.type === "class"
                ? (content.classNames || []).join("\n")
                : pattern.type === "config"
                  ? (content.configFiles || []).join("\n")
                  : pattern.type === "annotation"
                    ? (content.annotations || []).join("\n")
                    : allStrings

        for (const p of patterns) {
          let found = false

          if (p instanceof RegExp) {
            if (p.test(targetContent)) {
              found = true
            }
          } else if (typeof p === "string") {
            if (targetContent.includes(p.toLowerCase()) || targetContent.includes(p)) {
              found = true
            }
          }

          if (found) {
            matches.push(`${String(p)} (${pattern.confidence})`)
          }
        }
      }

      if (matches.length > 0) {
        results.set(tech.name, { tech, matches })
      }
    }
  }

  return results
}

/**
 * Get all technologies for a category
 */
export function getJavaFingerprintsByCategory(category: string): JavaFingerprint[] {
  return JAVA_FINGERPRINTS[category] || []
}

/**
 * Get all technology categories
 */
export function getJavaCategories(): string[] {
  return Object.keys(JAVA_FINGERPRINTS)
}
