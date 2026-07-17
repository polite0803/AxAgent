// SPDX-License-Identifier: AGPL-3.0-only

import { describe, expect, it } from "vitest";

import { generateUIFromNaturalLanguage } from "../dynamicUI/nl2ui";

describe("nl2ui", () => {
  describe("generateUIFromNaturalLanguage", () => {
    it("应返回包含 schema、title 和 description 的对象", () => {
      const result = generateUIFromNaturalLanguage("创建一个用户表单");
      expect(result).toHaveProperty("schema");
      expect(result).toHaveProperty("title");
      expect(result).toHaveProperty("description");
    });

    it("应返回合法的 schema 结构", () => {
      const result = generateUIFromNaturalLanguage("创建一个包含姓名和邮箱的表单");
      const schema = result.schema;
      expect(schema.version).toBe("1.0");
      expect(schema.type).toBe("Column");
      expect(Array.isArray(schema.children)).toBe(true);
      expect(schema.children!.length).toBeGreaterThanOrEqual(2);
    });

    it("应识别姓名字段", () => {
      const result = generateUIFromNaturalLanguage("输入姓名");
      const fields = result.schema.children!.find(
        (c) => c.type === "Form",
      )?.children;
      expect(fields).toBeDefined();
      const nameField = fields!.find((f) => f.props.name === "name");
      expect(nameField).toBeDefined();
      expect(nameField!.type).toBe("Input");
    });

    it("应识别邮箱字段", () => {
      const result = generateUIFromNaturalLanguage("请输入邮箱地址");
      const fields = result.schema.children!.find(
        (c) => c.type === "Form",
      )?.children;
      expect(fields).toBeDefined();
      const emailField = fields!.find((f) => f.props.name === "email");
      expect(emailField).toBeDefined();
      expect(emailField!.type).toBe("Input");
    });

    it("应识别性别字段为 Radio", () => {
      const result = generateUIFromNaturalLanguage("请选择性别");
      const fields = result.schema.children!.find(
        (c) => c.type === "Form",
      )?.children;
      const genderField = fields!.find((f) => f.props.name === "gender");
      expect(genderField).toBeDefined();
      expect(genderField!.type).toBe("Radio");
      expect(genderField!.props.options).toBeDefined();
    });

    it("应识别年龄字段为 Number", () => {
      const result = generateUIFromNaturalLanguage("请输入年龄");
      const fields = result.schema.children!.find(
        (c) => c.type === "Form",
      )?.children;
      const ageField = fields!.find((f) => f.props.name === "age");
      expect(ageField).toBeDefined();
      expect(ageField!.type).toBe("Number");
    });

    it("应识别日期字段为 DatePicker", () => {
      const result = generateUIFromNaturalLanguage("选择出生日期");
      const fields = result.schema.children!.find(
        (c) => c.type === "Form",
      )?.children;
      const dateField = fields!.find((f) => f.props.name === "date");
      expect(dateField).toBeDefined();
      expect(dateField!.type).toBe("DatePicker");
    });

    it("应识别标题字段为 required", () => {
      const result = generateUIFromNaturalLanguage("请输入标题");
      const fields = result.schema.children!.find(
        (c) => c.type === "Form",
      )?.children;
      const titleField = fields!.find((f) => f.props.name === "title");
      expect(titleField).toBeDefined();
      expect(titleField!.props.required).toBe(true);
    });

    it("无匹配关键词时应返回默认字段", () => {
      const result = generateUIFromNaturalLanguage("xyz");
      const fields = result.schema.children!.find(
        (c) => c.type === "Form",
      )?.children;
      expect(fields!.length).toBeGreaterThanOrEqual(2);
      expect(fields!.some((f) => f.props.name === "title")).toBe(true);
      expect(fields!.some((f) => f.props.name === "content")).toBe(true);
    });

    it("dashboard 关键词应生成 Chart 组件", () => {
      const result = generateUIFromNaturalLanguage("创建一个仪表盘");
      const fields = result.schema.children!.find(
        (c) => c.type === "Form",
      )?.children;
      expect(fields!.some((f) => f.type === "Chart")).toBe(true);
    });

    it("应检测中文标题", () => {
      const result = generateUIFromNaturalLanguage("创建一个叫做用户信息的表单");
      expect(result.title).toBe("用户信息的表单");
    });

    it("表单关键词应默认标题为'表单'", () => {
      const result = generateUIFromNaturalLanguage("创建一个表单");
      expect(result.title).toBe("表单");
    });

    it("description 应包含截断的 prompt", () => {
      const longPrompt = "A".repeat(100);
      const result = generateUIFromNaturalLanguage(longPrompt);
      expect(result.description).toContain("A");
      expect(result.description.length).toBeLessThan(longPrompt.length + 50);
    });

    it("多个关键词不应生成重复字段", () => {
      const result = generateUIFromNaturalLanguage("姓名 name 名字");
      const fields = result.schema.children!.find(
        (c) => c.type === "Form",
      )?.children;
      const nameFields = fields!.filter((f) => f.props.name === "name");
      expect(nameFields.length).toBe(1);
    });
  });
});
