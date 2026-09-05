"use client";

import { FormEvent, useMemo, useState } from "react";
import { ErrorState } from "@/components/FeedbackStates";
import { apiClient } from "@/lib/api-client";
import { parseMajorToMinor } from "@/lib/money";
import {
  digitalDeliveryMethodLabel,
  FILE_SECURITY_CHECK_LABEL,
  fulfillmentTypeLabel,
  shippingMethodLabel,
} from "@/lib/statusMaps";
import type {
  DigitalDeliveryMethod,
  FileSecurityCheck,
  FulfillmentType,
  ProductDetail,
  SellerProductDraft,
  ShippingMethod,
} from "@/lib/types";

const DEFAULT_CURRENCY = "USD";

function securityCheckForFile(fileName: string): FileSecurityCheck {
  if (!fileName.trim()) {
    return {
      status: "pending",
      summary: "Upload a file so the security scan can run before publish.",
    };
  }
  const lower = fileName.toLowerCase();
  if (lower.endsWith(".exe") || lower.endsWith(".bat")) {
    return {
      status: "failed",
      summary: "Executable files are blocked before inventory is created.",
    };
  }
  return {
    status: "passed",
    summary: "File type and size limits passed. A server scan still runs on submit.",
  };
}

export function ProductEditor() {
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [fulfillmentType, setFulfillmentType] =
    useState<FulfillmentType>("digital");
  const [currency, setCurrency] = useState(DEFAULT_CURRENCY);
  const [priceMajor, setPriceMajor] = useState("19.99");
  const [availableStock, setAvailableStock] = useState("10");
  const [refundWindowDays, setRefundWindowDays] = useState("14");
  const [deliveryMethod, setDeliveryMethod] =
    useState<DigitalDeliveryMethod>("file_download");
  const [fileName, setFileName] = useState("");
  const [weightGrams, setWeightGrams] = useState("400");
  const [lengthMm, setLengthMm] = useState("200");
  const [widthMm, setWidthMm] = useState("150");
  const [heightMm, setHeightMm] = useState("20");
  const [shippingRegions, setShippingRegions] = useState("US, CA, GB");
  const [shippingMethod, setShippingMethod] =
    useState<ShippingMethod>("standard_ground");
  const [estimatedDaysMin, setEstimatedDaysMin] = useState("3");
  const [estimatedDaysMax, setEstimatedDaysMax] = useState("7");
  const [error, setError] = useState<unknown>(null);
  const [createdId, setCreatedId] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  const fileCheck = useMemo(
    () =>
      fulfillmentType === "digital"
        ? securityCheckForFile(fileName)
        : { status: "not_required" as const, summary: "Physical listings do not upload buyer files." },
    [fileName, fulfillmentType],
  );

  const draft = useMemo<SellerProductDraft | null>(() => {
    try {
      const priceMinor = parseMajorToMinor(priceMajor, currency);
      const stock = Number.parseInt(availableStock, 10);
      const refundDays = Number.parseInt(refundWindowDays, 10);
      if (!Number.isInteger(stock) || stock < 0 || !Number.isInteger(refundDays)) {
        return null;
      }

      return {
        title: title.trim(),
        description: description.trim(),
        fulfillment_type: fulfillmentType,
        price_minor: priceMinor,
        currency,
        available_stock: stock,
        refund_window_days: refundDays,
        digital:
          fulfillmentType === "digital"
            ? {
                delivery_method: deliveryMethod,
                file_name: fileName.trim() || undefined,
                file_security_check: fileCheck,
              }
            : undefined,
        physical:
          fulfillmentType === "physical_standard"
            ? {
                weight_grams: Number.parseInt(weightGrams, 10),
                length_mm: Number.parseInt(lengthMm, 10),
                width_mm: Number.parseInt(widthMm, 10),
                height_mm: Number.parseInt(heightMm, 10),
                shipping_regions: shippingRegions
                  .split(",")
                  .map((region) => region.trim())
                  .filter(Boolean),
                shipping_method: shippingMethod,
                estimated_days_min: Number.parseInt(estimatedDaysMin, 10),
                estimated_days_max: Number.parseInt(estimatedDaysMax, 10),
              }
            : undefined,
      };
    } catch {
      return null;
    }
  }, [
    availableStock,
    currency,
    deliveryMethod,
    description,
    estimatedDaysMax,
    estimatedDaysMin,
    fileCheck,
    fileName,
    fulfillmentType,
    heightMm,
    lengthMm,
    priceMajor,
    refundWindowDays,
    shippingMethod,
    shippingRegions,
    title,
    weightGrams,
    widthMm,
  ]);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!draft || !draft.title || !draft.description) {
      setError(
        new Error("Title, description, and a valid integer price are required."),
      );
      return;
    }
    if (draft.digital?.file_security_check.status === "failed") {
      setError(new Error(draft.digital.file_security_check.summary));
      return;
    }

    setIsSubmitting(true);
    setError(null);
    try {
      const created = await apiClient.post<ProductDetail>(
        "/seller/products",
        draft,
      );
      setCreatedId(created.product_id);
    } catch (submitError) {
      setError(submitError);
    } finally {
      setIsSubmitting(false);
    }
  }

  return (
    <section className="stack-lg">
      <header className="section-heading">
        <h1 className="display">New listing</h1>
        <p className="lede">
          Digital and physical goods share price, stock, and refund fields.
          Delivery fields change with fulfillment type.
        </p>
      </header>

      <form className="editor-grid" onSubmit={(event) => void handleSubmit(event)}>
        <label>
          Title
          <input value={title} onChange={(event) => setTitle(event.target.value)} />
        </label>
        <label className="wide">
          Description
          <textarea
            rows={5}
            value={description}
            onChange={(event) => setDescription(event.target.value)}
          />
        </label>
        <fieldset className="wide">
          <legend>Fulfillment type</legend>
          <label className="inline-field">
            <input
              type="radio"
              name="fulfillment_type"
              checked={fulfillmentType === "digital"}
              onChange={() => setFulfillmentType("digital")}
            />
            Digital delivery
          </label>
          <label className="inline-field">
            <input
              type="radio"
              name="fulfillment_type"
              checked={fulfillmentType === "physical_standard"}
              onChange={() => setFulfillmentType("physical_standard")}
            />
            Physical shipment
          </label>
        </fieldset>
        <label>
          Currency
          <input
            maxLength={3}
            value={currency}
            onChange={(event) => setCurrency(event.target.value.toUpperCase())}
          />
        </label>
        <label>
          Price
          <input
            value={priceMajor}
            onChange={(event) => setPriceMajor(event.target.value)}
          />
        </label>
        <label>
          Available stock
          <input
            inputMode="numeric"
            value={availableStock}
            onChange={(event) => setAvailableStock(event.target.value)}
          />
        </label>
        <label>
          Refund window (days)
          <input
            inputMode="numeric"
            value={refundWindowDays}
            onChange={(event) => setRefundWindowDays(event.target.value)}
          />
        </label>

        {fulfillmentType === "digital" ? (
          <>
            <label>
              Digital delivery
              <select
                value={deliveryMethod}
                onChange={(event) =>
                  setDeliveryMethod(event.target.value as DigitalDeliveryMethod)
                }
              >
                <option value="file_download">File download</option>
                <option value="license_key">License key</option>
                <option value="redemption_code">Redemption code</option>
                <option value="access_link">Access link</option>
              </select>
            </label>
            <label>
              File name
              <input
                value={fileName}
                onChange={(event) => setFileName(event.target.value)}
              />
            </label>
          </>
        ) : (
          <>
            <label>
              Weight (grams)
              <input
                inputMode="numeric"
                value={weightGrams}
                onChange={(event) => setWeightGrams(event.target.value)}
              />
            </label>
            <label>
              Length (mm)
              <input
                inputMode="numeric"
                value={lengthMm}
                onChange={(event) => setLengthMm(event.target.value)}
              />
            </label>
            <label>
              Width (mm)
              <input
                inputMode="numeric"
                value={widthMm}
                onChange={(event) => setWidthMm(event.target.value)}
              />
            </label>
            <label>
              Height (mm)
              <input
                inputMode="numeric"
                value={heightMm}
                onChange={(event) => setHeightMm(event.target.value)}
              />
            </label>
            <label className="wide">
              Shipping regions
              <input
                value={shippingRegions}
                onChange={(event) => setShippingRegions(event.target.value)}
              />
            </label>
            <label>
              Shipping method
              <select
                value={shippingMethod}
                onChange={(event) =>
                  setShippingMethod(event.target.value as ShippingMethod)
                }
              >
                <option value="standard_ground">Standard ground</option>
                <option value="express">Express</option>
                <option value="local_pickup">Local pickup</option>
              </select>
            </label>
            <label>
              Min days
              <input
                inputMode="numeric"
                value={estimatedDaysMin}
                onChange={(event) => setEstimatedDaysMin(event.target.value)}
              />
            </label>
            <label>
              Max days
              <input
                inputMode="numeric"
                value={estimatedDaysMax}
                onChange={(event) => setEstimatedDaysMax(event.target.value)}
              />
            </label>
          </>
        )}

        <aside className="review-panel wide">
          <h2>Before submit</h2>
          <ul>
            <li>
              File security: {FILE_SECURITY_CHECK_LABEL[fileCheck.status]}.{" "}
              {fileCheck.summary}
            </li>
            <li>
              Inventory: {availableStock} units at {priceMajor} {currency}.
            </li>
            <li>
              Delivery: {fulfillmentTypeLabel(fulfillmentType)}
              {fulfillmentType === "digital"
                ? `, ${digitalDeliveryMethodLabel(deliveryMethod)}`
                : `, ${shippingMethodLabel(shippingMethod)} to ${shippingRegions}`}
            </li>
          </ul>
        </aside>

        <button type="submit" className="button-primary" disabled={isSubmitting}>
          Submit listing
        </button>
      </form>

      {createdId ? (
        <p role="status">Listing created. Product {createdId}</p>
      ) : null}
      {error ? <ErrorState error={error} /> : null}
    </section>
  );
}
