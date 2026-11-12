<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { open } from "@tauri-apps/plugin-dialog";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { commands } from "$lib/bindings";

  let signaturePath = $state<string | null>(null);
  let isUploading = $state(false);
  let errorMessage = $state("");
  let successMessage = $state("");

  let plumberName = $state("");
  let plumberPhone = $state("");
  let plumberAddress = $state("");
  let isSavingInfo = $state(false);

  onMount(async () => {
    const nameRes = await commands.getSetting("plumber_name");
    if (nameRes.status === "ok" && nameRes.data) plumberName = nameRes.data;

    const phoneRes = await commands.getSetting("plumber_phone");
    if (phoneRes.status === "ok" && phoneRes.data) plumberPhone = phoneRes.data;

    const addressRes = await commands.getSetting("plumber_address");
    if (addressRes.status === "ok" && addressRes.data)
      plumberAddress = addressRes.data;

    const sigRes = await commands.getSignaturePath();
    if (sigRes.status === "ok" && sigRes.data) {
      signaturePath = sigRes.data;
    }
  });

  function mapError(err: any) {
    const errStr = JSON.stringify(err);
    if (errStr.includes("InvalidLatinText"))
      return "جميع الحقول يجب أن تكون بالأحرف اللاتينية فقط (مثل: Ahmed).";
    if (errStr.includes("EmptyField"))
      return "الرجاء ملء جميع الحقول المطلوبة.";
    if (errStr.includes("IoError")) return "خطأ في النظام.";
    if (errStr.includes("DatabaseError")) return "خطأ في قاعدة البيانات.";
    return "فشل حفظ البيانات.";
  }

  async function savePlumberInfo() {
    isSavingInfo = true;
    errorMessage = "";
    successMessage = "";
    try {
      const nameRes = await commands.setSetting("plumber_name", plumberName);
      if (nameRes.status === "error") {
        errorMessage = mapError(nameRes.error);
        return;
      }

      const phoneRes = await commands.setSetting("plumber_phone", plumberPhone);
      if (phoneRes.status === "error") {
        errorMessage = mapError(phoneRes.error);
        return;
      }

      const addressRes = await commands.setSetting(
        "plumber_address",
        plumberAddress,
      );
      if (addressRes.status === "error") {
        errorMessage = mapError(addressRes.error);
        return;
      }

      successMessage = "تم حفظ بيانات السباك بنجاح";
    } catch (e) {
      errorMessage = "فشل حفظ البيانات";
    } finally {
      isSavingInfo = false;
    }
  }

  async function handleUpload() {
    try {
      const selectedPath = await open({
        filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg"] }],
      });
      if (!selectedPath) return;

      isUploading = true;
      errorMessage = "";
      successMessage = "";

      const res = await commands.saveSignature(selectedPath as string);
      if (res.status === "ok") {
        signaturePath = res.data;
        successMessage = "تم رفع التوقيع بنجاح";
      } else {
        errorMessage = "فشل رفع التوقيع: " + JSON.stringify(res.error);
      }
    } catch (e: any) {
      errorMessage = `فشل رفع التوقيع: ${e.message || String(e)}`;
    } finally {
      isUploading = false;
    }
  }
</script>

<div class="container">
  <div class="header">
    <h1>الإعدادات</h1>
    <button class="back-btn" onclick={() => goto("/")}>← رجوع</button>
  </div>

  {#if errorMessage}
    <div class="error">{errorMessage}</div>
  {/if}
  {#if successMessage}
    <div class="success">{successMessage}</div>
  {/if}

  <div class="form-section">
    <h2>بيانات السباك</h2>
    <div class="form-group">
      <label>اسم السباك / الشركة</label>
      <input
        type="text"
        bind:value={plumberName}
        placeholder="مثال: Plomberie Ahmed"
        dir="ltr"
      />
      <small
        style="color: #d16a6c; font-size: 0.8rem; margin-top: 4px; display: block;"
      >
        أحرف لاتينية فقط.
      </small>
    </div>
    <div class="form-group">
      <label>رقم الهاتف</label>
      <input
        type="text"
        bind:value={plumberPhone}
        placeholder="0555 55 55 55"
        dir="ltr"
      />
      <small
        style="color: #d16a6c; font-size: 0.8rem; margin-top: 4px; display: block;"
      >
        أحرف لاتينية فقط.
      </small>
    </div>
    <div class="form-group">
      <label>العنوان</label>
      <input
        type="text"
        bind:value={plumberAddress}
        placeholder="Alger, Algérie"
        dir="ltr"
      />
      <small
        style="color: #d16a6c; font-size: 0.8rem; margin-top: 4px; display: block;"
      >
        أحرف لاتينية فقط.
      </small>
    </div>
    <button
      class="btn-primary"
      onclick={savePlumberInfo}
      disabled={isSavingInfo}
    >
      {isSavingInfo ? "⏳ جاري الحفظ..." : "حفظ البيانات"}
    </button>
  </div>

  <div class="form-section" style="margin-top: 1rem;">
    <h2>صورة التوقيع</h2>
    <p class="desc">قم برفع صورة توقيعك (يفضل بخلفية بيضاء).</p>
    <button class="btn-primary" onclick={handleUpload} disabled={isUploading}>
      {isUploading ? "⏳ جاري الرفع..." : "📷 رفع صورة التوقيع"}
    </button>

    {#if signaturePath}
      <div class="preview-box">
        <p class="preview-label">معاينة التوقيع:</p>
        <img
          src={convertFileSrc(signaturePath)}
          alt="التوقيع"
          class="signature-img"
        />
      </div>
    {/if}
  </div>
</div>

<style>
  .container {
    max-width: 700px;
    margin: 0 auto;
    padding: 1rem;
    direction: rtl;
    color: #6c6d6c;
  }
  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
  }
  h1,
  h2 {
    color: #6f3c39;
    margin: 0;
  }
  h1 {
    font-size: 1.6rem;
    font-weight: 800;
  }
  h2 {
    font-size: 1.1rem;
    font-weight: 700;
    margin-bottom: 0.5rem;
  }
  .back-btn {
    background: #6f3c39;
    color: white;
    border: 2px solid #6f3c39;
    padding: 0.45rem 0.9rem;
    border-radius: 6px;
    cursor: pointer;
    font-weight: 700;
    box-shadow: 3px 3px 0px 0px #1a1a1a;
  }
  .form-section {
    background: #fdfcf8;
    padding: 1rem;
    border-radius: 8px;
    border: 2px solid #6f3c39;
    box-shadow: 4px 4px 0px 0px #6f3c39;
    margin-bottom: 1rem;
  }
  .form-group {
    margin-bottom: 0.85rem;
  }
  label {
    display: block;
    font-weight: 700;
    margin-bottom: 0.3rem;
    font-size: 0.85rem;
  }
  .desc {
    margin-bottom: 0.85rem;
    font-size: 0.85rem;
  }
  input {
    width: 100%;
    padding: 0.5rem;
    border: 2px solid #6f3c39;
    border-radius: 6px;
    font-size: 0.85rem;
    background: #e3e3db;
    color: #6c6d6c;
    font-weight: 500;
  }
  input:focus {
    outline: none;
    background: #ffffff;
    box-shadow: 3px 3px 0px 0px rgba(241, 67, 69, 0.3);
  }
  .btn-primary {
    background: #f14345;
    color: white;
    border: 2px solid #6f3c39;
    padding: 0.6rem 1.2rem;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.9rem;
    font-weight: 700;
    box-shadow: 3px 3px 0px 0px #6f3c39;
    margin-top: 0.5rem;
  }
  .btn-primary:active:not(:disabled) {
    transform: translate(3px, 3px);
    box-shadow: 0px 0px 0px 0px #6f3c39;
  }
  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .error {
    background: #fee;
    color: #c33;
    padding: 0.7rem;
    border-radius: 6px;
    margin-bottom: 1rem;
    border: 2px solid #c33;
    font-weight: 600;
    font-size: 0.85rem;
  }
  .success {
    background: #efe;
    color: #363;
    padding: 0.7rem;
    border-radius: 6px;
    margin-bottom: 1rem;
    border: 2px solid #363;
    font-weight: 600;
    font-size: 0.85rem;
  }
  .preview-box {
    margin-top: 1rem;
    padding: 0.75rem;
    background: #ffffff;
    border: 2px dashed #6f3c39;
    border-radius: 6px;
    text-align: center;
  }
  .preview-label {
    font-weight: 700;
    margin-bottom: 0.5rem;
    font-size: 0.85rem;
  }
  .signature-img {
    max-width: 100%;
    max-height: 100px;
    border: 2px solid #6f3c39;
    border-radius: 4px;
    background: white;
    mix-blend-mode: multiply;
  }
</style>
